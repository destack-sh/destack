use std::path::Path;

use super::{CacheMetadata, CacheStore, CacheStoreError};

/// Cache store that disables persistence.
#[derive(Debug, Default, Clone)]
pub struct NullCacheStore;

impl NullCacheStore {
    /// Create a null cache store.
    pub fn new() -> Self {
        Self
    }
}

impl CacheStore for NullCacheStore {
    fn with_lock(&self, _path: &Path, operation: &mut dyn FnMut()) -> Result<(), CacheStoreError> {
        operation();

        Ok(())
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Ok(None)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn touch(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn exists(&self, _path: &Path) -> Result<bool, CacheStoreError> {
        Ok(false)
    }

    fn list(&self, _path: &Path) -> Result<Vec<std::path::PathBuf>, CacheStoreError> {
        Ok(Vec::new())
    }

    fn remove(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn metadata(&self, _path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        Ok(None)
    }
}
