use std::collections::HashSet;

use destack_core::StringPool;

use crate::{ArtifactError, ArtifactRecord, ArtifactVersion};

/// Persistent store for artifacts.
pub trait ArtifactStore: std::fmt::Debug + Send + Sync {
    /// Load one artifact by exact version.
    fn load(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError>;

    /// Queue one artifact for persistence.
    fn store(&self, record: ArtifactRecord) -> Result<(), ArtifactError>;

    /// Persist queued artifacts.
    fn flush(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactError>;

    /// Retain only selected artifact versions.
    fn retain(
        &self,
        versions: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError>;
}

/// Rows and bytes published by one artifact flush.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArtifactFlush {
    /// The number of segment files published.
    pub segments: usize,
    /// The number of artifacts published.
    pub artifacts: usize,
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
        _version: &ArtifactVersion,
        _strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError> {
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
        _versions: &HashSet<ArtifactVersion>,
        _strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        Ok(())
    }
}
