use prost::Message;
use risingwave_pb::hummock::{HummockContextPinnedSnapshot, HummockContextRefId};
use risingwave_storage::hummock::HummockEpoch;

use crate::hummock::model::Transactional;
use crate::model::MetadataModel;
use crate::storage::{ColumnFamilyUtils, Operation, Transaction};

/// Column family name for hummock context pinned snapshot
/// `cf(hummock_context_pinned_snapshot)`: `HummockContextRefId` -> `HummockContextPinnedSnapshot`
const HUMMOCK_CONTEXT_PINNED_SNAPSHOT_CF_NAME: &str = "cf/hummock_context_pinned_snapshot";

impl MetadataModel for HummockContextPinnedSnapshot {
    type ProstType = HummockContextPinnedSnapshot;
    type KeyType = HummockContextRefId;

    fn cf_name() -> String {
        String::from(HUMMOCK_CONTEXT_PINNED_SNAPSHOT_CF_NAME)
    }

    fn to_protobuf(&self) -> Self::ProstType {
        self.clone()
    }

    fn from_protobuf(prost: Self::ProstType) -> Self {
        prost
    }

    fn key(&self) -> risingwave_common::error::Result<Self::KeyType> {
        Ok(HummockContextRefId {
            id: self.context_id,
        })
    }
}

pub trait HummockContextPinnedSnapshotExt {
    fn pin_snapshot(&mut self, new_snapshot_id: HummockEpoch);

    fn unpin_snapshot(&mut self, pinned_snapshot_id: HummockEpoch);

    fn update(&self, trx: &mut Transaction);
}

impl HummockContextPinnedSnapshotExt for HummockContextPinnedSnapshot {
    fn pin_snapshot(&mut self, epoch: HummockEpoch) {
        let found = self.snapshot_id.iter().position(|&v| v == epoch);
        if found.is_none() {
            self.snapshot_id.push(epoch);
        }
    }

    fn unpin_snapshot(&mut self, epoch: HummockEpoch) {
        let found = self.snapshot_id.iter().position(|&v| v == epoch);
        if let Some(pos) = found {
            self.snapshot_id.remove(pos);
        }
    }

    fn update(&self, trx: &mut Transaction) {
        if self.snapshot_id.is_empty() {
            self.delete(trx);
        } else {
            self.upsert(trx);
        }
    }
}

impl Transactional for HummockContextPinnedSnapshot {
    fn upsert(&self, trx: &mut Transaction) {
        trx.add_operations(vec![Operation::Put(
            ColumnFamilyUtils::prefix_key_with_cf(
                self.key().unwrap().encode_to_vec(),
                HummockContextPinnedSnapshot::cf_name(),
            ),
            self.encode_to_vec(),
            None,
        )]);
    }

    fn delete(&self, trx: &mut Transaction) {
        trx.add_operations(vec![Operation::Delete(
            ColumnFamilyUtils::prefix_key_with_cf(
                self.key().unwrap().encode_to_vec(),
                HummockContextPinnedSnapshot::cf_name(),
            ),
            None,
        )]);
    }
}