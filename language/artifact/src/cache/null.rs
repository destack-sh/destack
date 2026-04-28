use std::path::Path;

use super::{CacheStore, CacheStoreError};

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
    fn with_exclusive_lock(
        &self,
        _path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), CacheStoreError> {
        operation();

        Ok(())
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Ok(None)
    }

    fn write_once(&self, _path: &Path, _bytes: &[u8]) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn byte_len(&self, _path: &Path) -> Result<Option<u64>, CacheStoreError> {
        Ok(None)
    }
}
