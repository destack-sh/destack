use std::collections::HashSet;

use destack_core::StringPool;

use crate::{ArtifactRecord, ArtifactStoreError, ArtifactTable, ArtifactVersion, BlobStoreError};

/// Persistent store for exact artifact records.
pub trait ArtifactStore: std::fmt::Debug + Send + Sync {
    /// Load one exact artifact record when it is present.
    fn load(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactStoreError>;

    /// Queue one exact artifact record when this store persists artifacts.
    fn store(
        &self,
        version: &ArtifactVersion,
        table: &ArtifactTable,
        strings: &StringPool,
    ) -> Result<(), ArtifactStoreError>;

    /// Persist queued artifact records.
    fn flush(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactStoreError>;

    /// Retain only reachable artifact records.
    fn retain(
        &self,
        reachable: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactStoreError>;
}

/// Records and bytes published by one artifact flush.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArtifactFlush {
    /// The number of segment files published.
    pub segments: usize,
    /// The number of artifact records published.
    pub records: usize,
    /// The number of interned strings carried by the published segments.
    pub strings: usize,
    /// The number of encoded bytes published.
    pub bytes: usize,
}

/// Artifact store that disables persistence.
#[derive(Debug, Default, Clone)]
pub struct NullArtifactStore;

impl NullArtifactStore {
    /// Create a null artifact store.
    pub fn new() -> Self {
        Self
    }
}

impl ArtifactStore for NullArtifactStore {
    fn load(
        &self,
        _expected: &ArtifactVersion,
        _strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactStoreError> {
        Ok(None)
    }

    fn store(
        &self,
        _version: &ArtifactVersion,
        _table: &ArtifactTable,
        _strings: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        Ok(())
    }

    fn flush(&self, _strings: &StringPool) -> Result<ArtifactFlush, ArtifactStoreError> {
        Ok(ArtifactFlush::default())
    }

    fn retain(
        &self,
        _reachable: &HashSet<ArtifactVersion>,
        _strings: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        Ok(())
    }
}

impl From<BlobStoreError> for ArtifactStoreError {
    fn from(error: BlobStoreError) -> Self {
        Self::Store(Box::new(error))
    }
}
