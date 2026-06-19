use std::path::Path;

use super::{BlobStore, BlobStoreError};

/// Blob store that disables persistence.
#[derive(Debug, Default, Clone)]
pub struct NullBlobStore;

impl NullBlobStore {
    /// Create a null blob store.
    pub fn new() -> Self {
        Self
    }
}

impl BlobStore for NullBlobStore {
    fn with_exclusive_lock(
        &self,
        _path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), BlobStoreError> {
        operation();

        Ok(())
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError> {
        Ok(None)
    }

    fn write_once(&self, _path: &Path, _bytes: &[u8]) -> Result<(), BlobStoreError> {
        Ok(())
    }

    fn byte_len(&self, _path: &Path) -> Result<Option<u64>, BlobStoreError> {
        Ok(None)
    }

    fn entries(&self, _root: &Path) -> Result<Vec<std::path::PathBuf>, BlobStoreError> {
        Ok(Vec::new())
    }

    fn remove(&self, _path: &Path) -> Result<(), BlobStoreError> {
        Ok(())
    }
}
