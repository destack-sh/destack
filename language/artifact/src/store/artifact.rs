use std::collections::HashSet;

use destack_core::StringPool;

use crate::{
    ArtifactBindingRecord, ArtifactError, ArtifactInput, ArtifactRecord, ArtifactResultRecord,
    ArtifactVersion,
};

/// Persistent store for artifact bindings and results.
pub trait ArtifactStore: std::fmt::Debug + Send + Sync {
    /// Load one artifact binding by exact input.
    fn load_binding(
        &self,
        input: &ArtifactInput,
        strings: &StringPool,
    ) -> Result<Option<ArtifactBindingRecord>, ArtifactError>;

    /// Load one exact artifact result when it is present.
    fn load_result(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactResultRecord>, ArtifactError>;

    /// Queue one artifact binding and result for persistence.
    fn store(&self, record: ArtifactRecord) -> Result<(), ArtifactError>;

    /// Persist queued artifact bindings and results.
    fn flush(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactError>;

    /// Retain only selected artifact input records.
    fn retain(
        &self,
        inputs: &HashSet<ArtifactInput>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError>;
}

/// Rows and bytes published by one artifact flush.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArtifactFlush {
    /// The number of segment files published.
    pub segments: usize,
    /// The number of artifact results published.
    pub results: usize,
    /// The number of artifact bindings published.
    pub bindings: usize,
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
    fn load_binding(
        &self,
        _input: &ArtifactInput,
        _strings: &StringPool,
    ) -> Result<Option<ArtifactBindingRecord>, ArtifactError> {
        Ok(None)
    }

    fn load_result(
        &self,
        _expected: &ArtifactVersion,
        _strings: &StringPool,
    ) -> Result<Option<ArtifactResultRecord>, ArtifactError> {
        Ok(None)
    }

    fn store(&self, _record: ArtifactRecord) -> Result<(), ArtifactError> {
        Ok(())
    }

    fn flush(&self, _strings: &StringPool) -> Result<ArtifactFlush, ArtifactError> {
        Ok(ArtifactFlush::default())
    }

    fn retain(
        &self,
        _inputs: &HashSet<ArtifactInput>,
        _strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        Ok(())
    }
}
