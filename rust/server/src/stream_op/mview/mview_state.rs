use std::collections::HashMap;

use super::{serialize_cell, serialize_cell_idx, serialize_pk};

use crate::stream_op::keyspace::StateStore;

use bytes::Bytes;
use risingwave_common::array::Row;
use risingwave_common::catalog::Schema;
use risingwave_common::error::{ErrorCode, Result};

/// `ManagedMviewState` buffers recent mutations. Data will be written
/// to backend storage on calling `flush`.
pub struct ManagedMViewState<S: StateStore> {
    prefix: Vec<u8>,
    schema: Schema,
    pk_columns: Vec<usize>,
    memtable: HashMap<Row, Option<Row>>,
    storage: S,
}

impl<S: StateStore> ManagedMViewState<S> {
    pub fn new(prefix: Vec<u8>, schema: Schema, pk_columns: Vec<usize>, storage: S) -> Self {
        Self {
            prefix,
            schema,
            pk_columns,
            memtable: HashMap::new(),
            storage,
        }
    }

    pub fn put(&mut self, pk: Row, value: Row) {
        self.memtable.insert(pk, Some(value));
    }

    pub fn delete(&mut self, pk: Row) {
        self.memtable.insert(pk, None);
    }

    // TODO(MrCroxx): flush can be performed in another coruntine by taking the snapshot of
    // memtable.
    pub async fn flush(&mut self) -> Result<()> {
        let mut batch = Vec::with_capacity(self.memtable.len() * self.schema.len());

        for (pk, cells) in self.memtable.drain() {
            debug_assert_eq!(self.pk_columns.len(), pk.0.len());
            let pk_buf = serialize_pk(&pk)?;
            for cell_idx in 0..self.schema.len() {
                // TODO(MrCroxx): More efficient encoding is needed.
                let key = [
                    &self.prefix[..],
                    &pk_buf[..],
                    &serialize_cell_idx(cell_idx as i32)?[..],
                ]
                .concat();
                let value = match &cells {
                    Some(cells) => Some(serialize_cell(&cells[cell_idx])?),
                    None => None,
                };
                batch.push((key.into(), value.map(Bytes::from)));
            }
        }

        self.storage
            .ingest_batch(batch)
            .await
            .map_err(|err| ErrorCode::InternalError(err.to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use risingwave_common::catalog::Field;
    use risingwave_common::types::{Int32Type, Scalar};

    use crate::stream_op::MemoryStateStore;

    use super::*;

    #[tokio::test]
    async fn test_mview_state() {
        // Only assert pk and columns can be successfully put/delete/flush,
        // and the ammount of rows is expected.
        let state_store = MemoryStateStore::default();
        let schema = Schema::new(vec![
            Field::new(Int32Type::create(false)),
            Field::new(Int32Type::create(false)),
        ]);
        let pk_columns = vec![0];
        let prefix = b"test-prefix-42".to_vec();
        let mut state =
            ManagedMViewState::new(prefix.clone(), schema, pk_columns, state_store.clone());

        state.put(
            Row(vec![Some(1_i32.to_scalar_value())]),
            Row(vec![
                Some(1_i32.to_scalar_value()),
                Some(11_i32.to_scalar_value()),
            ]),
        );
        state.put(
            Row(vec![Some(2_i32.to_scalar_value())]),
            Row(vec![
                Some(2_i32.to_scalar_value()),
                Some(22_i32.to_scalar_value()),
            ]),
        );
        state.put(
            Row(vec![Some(3_i32.to_scalar_value())]),
            Row(vec![
                Some(3_i32.to_scalar_value()),
                Some(33_i32.to_scalar_value()),
            ]),
        );
        state.delete(Row(vec![Some(2_i32.to_scalar_value())]));

        state.flush().await.unwrap();
        let data = state_store.scan(&prefix[..], None).await.unwrap();
        // cell-based storage has 4 cells
        assert_eq!(data.len(), 4);

        state.delete(Row(vec![Some(3_i32.to_scalar_value())]));
        state.flush().await.unwrap();
        let data = state_store.scan(&prefix[..], None).await.unwrap();
        assert_eq!(data.len(), 2);
    }
}