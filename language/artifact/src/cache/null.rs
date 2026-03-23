use std::path::Path;

use super::{CacheLock, CacheMetadata, CacheStore, CacheStoreError, CacheStoreKind};

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
    fn kind(&self) -> CacheStoreKind {
        CacheStoreKind::Null
    }

    fn lock_shared(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Ok(CacheLock::Noop)
    }

    fn lock_exclusive(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Ok(CacheLock::Noop)
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Ok(None)
    }

    fn read_prefix(&self, _path: &Path, _limit: usize) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Ok(None)
    }

    fn write_atomic(&self, _path: &Path, _bytes: &[u8]) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn touch(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn exists(&self, _path: &Path) -> Result<bool, CacheStoreError> {
        Ok(false)
    }

    fn remove(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Ok(())
    }

    fn metadata(&self, _path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::NullCacheStore;
    use crate::CacheStore;

    #[test]
    fn test_null_cache_store_is_empty() {
        let store = NullCacheStore::new();
        let path = PathBuf::from("/cache/null.bin");

        let _lock = store.lock_shared(&path).unwrap();
        store.write_atomic(&path, b"hello").unwrap();

        assert!(store.read(&path).unwrap().is_none());
        assert!(!store.exists(&path).unwrap());
        assert!(store.metadata(&path).unwrap().is_none());
    }
}
