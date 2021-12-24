use bytes::{BufMut, Bytes};
use risingwave_common::catalog::TableId;
use risingwave_common::error::Result;

use crate::StateStore;

/// Represents a unit part of [`Keyspace`].
#[derive(Clone, Debug)]
pub enum Segment {
    /// Segment with fixed length can be encoded directly.
    FixedLength(Vec<u8>),

    /// Segment with variant length should be prepended the length of itself.
    VariantLength(Vec<u8>),
}

impl Segment {
    pub fn u16(id: u16) -> Self {
        Self::FixedLength(id.to_be_bytes().to_vec())
    }

    pub fn u32(id: u32) -> Self {
        Self::FixedLength(id.to_be_bytes().to_vec())
    }

    /// Encode this segment to a mutable buffer.
    pub fn encode(&self, buf: &mut impl BufMut) {
        match self {
            Segment::FixedLength(fixed) => buf.put(fixed.as_slice()),
            Segment::VariantLength(variant) => {
                buf.put_u16(
                    variant
                        .len()
                        .try_into()
                        .expect("segment length out of u16 range"),
                );
                buf.put_slice(variant.as_slice());
            }
        }
    }
}

/// Provides API to read key-value pairs of a prefix in the storage backend.
#[derive(Clone)]
pub struct Keyspace<S: StateStore> {
    store: S,

    /// Encoded representation for all segments.
    prefix: Vec<u8>,
}

impl<S: StateStore> Keyspace<S> {
    /// Create a root [`Keyspace`] for an executor.
    pub fn executor_root(store: S, id: u32) -> Self {
        let mut root = Self {
            store,
            prefix: Vec::with_capacity(5),
        };
        root.push(Segment::FixedLength(b"e".to_vec()));
        root.push(Segment::u32(id));
        root
    }

    /// Create a root [`Keyspace`] for a table.
    pub fn table_root(store: S, id: &TableId) -> Self {
        let mut root = Self {
            store,
            prefix: Vec::new(),
        };
        root.push(Segment::FixedLength(b"t".to_vec()));
        root.push(Segment::VariantLength(format!("{:?}", id).into())); // TODO: this should be replaced with more-reasonable representation
        root
    }

    /// Push a [`Segment`] to this keyspace.
    pub fn push(&mut self, segment: Segment) {
        segment.encode(&mut self.prefix);
    }

    /// Treat the keyspace as a single key, and return the key.
    pub fn key(&self) -> &[u8] {
        &self.prefix
    }

    /// Treat the keyspace as a single key, and get its value.
    pub async fn value(&self) -> Result<Option<Bytes>> {
        self.store.get(&self.prefix).await
    }

    /// Concatenate this keyspace and the given key to produce a prefixed key.
    pub fn prefixed_key(&self, key: impl AsRef<[u8]>) -> Vec<u8> {
        [self.prefix.as_slice(), key.as_ref()].concat()
    }

    /// Get from the keyspace with the `prefixed_key` of given key.
    pub async fn get(&self, key: impl AsRef<[u8]>) -> Result<Option<Bytes>> {
        self.store.get(&self.prefixed_key(key)).await
    }

    /// Scan `limit` keys from the keyspace and get their values. If `limit` is None, all keys of
    /// the given prefix will be scanned.
    pub async fn scan(&self, limit: Option<usize>) -> Result<Vec<(Bytes, Bytes)>> {
        self.store.scan(&self.prefix, limit).await
    }

    /// Scan from the keyspace, and then strip the prefix of this keyspace.
    ///
    /// See also: [`Keyspace::scan`]
    pub async fn scan_strip_prefix(&self, limit: Option<usize>) -> Result<Vec<(Bytes, Bytes)>> {
        let mut pairs = self.scan(limit).await?;
        pairs
            .iter_mut()
            .for_each(|(k, _v)| *k = k.slice(self.prefix.len()..));
        Ok(pairs)
    }

    /// Get an iterator with the prefix of this keyspace.
    pub fn iter(&self) -> S::Iter {
        self.store.iter(self.key())
    }

    /// Get the underlying state store.
    pub fn state_store(&self) -> S {
        self.store.clone()
    }

    /// Get a sub-keyspace by pushing a [`Segment`].
    #[must_use]
    pub fn with_segment(&self, segment: Segment) -> Self {
        let mut new_keyspace = self.clone();
        new_keyspace.push(segment);
        new_keyspace
    }
}