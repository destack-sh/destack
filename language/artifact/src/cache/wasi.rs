use std::io::ErrorKind;
use std::path::Path;

use crate::cache::{CacheLock, CacheMetadata, CacheStore, CacheStoreError, CacheStoreKind};

/// Cache store backed by disk on unsupported targets.
#[derive(Debug, Default, Clone)]
pub struct DiskCacheStore;

impl DiskCacheStore {
    /// Create a disk cache store.
    pub fn new() -> Self {
        Self
    }
}

impl CacheStore for DiskCacheStore {
    fn kind(&self) -> CacheStoreKind {
        CacheStoreKind::Disk
    }

    fn lock_shared(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Err(unsupported())
    }

    fn lock_exclusive(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Err(unsupported())
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Err(unsupported())
    }

    fn read_prefix(&self, _path: &Path, _limit: usize) -> Result<Option<Vec<u8>>, CacheStoreError> {
        Err(unsupported())
    }

    fn write_atomic(&self, _path: &Path, _bytes: &[u8]) -> Result<(), CacheStoreError> {
        Err(unsupported())
    }

    fn touch(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Err(unsupported())
    }

    fn exists(&self, _path: &Path) -> Result<bool, CacheStoreError> {
        Err(unsupported())
    }

    fn remove(&self, _path: &Path) -> Result<(), CacheStoreError> {
        Err(unsupported())
    }

    fn metadata(&self, _path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        Err(unsupported())
    }
}

/// Build one unsupported cache-store error.
fn unsupported() -> CacheStoreError {
    CacheStoreError::Io(std::io::Error::new(
        ErrorKind::Unsupported,
        "disk cache is not supported on this target",
    ))
}
