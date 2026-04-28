use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;

use super::{CacheStore, CacheStoreError};

/// Cache entry stored in memory.
#[derive(Debug, Clone)]
struct MemoryCacheEntry {
    /// Cached bytes.
    bytes: Vec<u8>,
}

/// Cache store backed by memory.
#[derive(Debug, Default)]
pub struct MemoryCacheStore {
    /// Cached entries keyed by path.
    entries: RwLock<HashMap<PathBuf, MemoryCacheEntry>>,
    /// Lock guard for shared and exclusive cache access.
    lock: RwLock<()>,
}

impl MemoryCacheStore {
    /// Create a memory cache store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CacheStore for MemoryCacheStore {
    fn with_exclusive_lock(
        &self,
        _path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), CacheStoreError> {
        let _lock = self.lock.write();

        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        let entries = self.entries.read();
        Ok(entries.get(path).map(|entry| entry.bytes.clone()))
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
        let entry = MemoryCacheEntry {
            bytes: bytes.to_vec(),
        };
        let mut entries = self.entries.write();
        if entries.contains_key(path) {
            return Err(CacheStoreError::AlreadyExists);
        }

        entries.insert(path.to_path_buf(), entry);
        Ok(())
    }

    fn byte_len(&self, path: &Path) -> Result<Option<u64>, CacheStoreError> {
        let entries = self.entries.read();
        let entry = match entries.get(path) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        Ok(Some(entry.bytes.len() as u64))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::MemoryCacheStore;
    use crate::CacheStore;

    #[test]
    fn test_memory_cache_store_roundtrip() {
        let store = MemoryCacheStore::new();
        let path = PathBuf::from("/cache/path.bin");

        // locked write
        store
            .with_exclusive_lock(&path, &mut || {
                store.write_once(&path, b"hello").unwrap();
            })
            .unwrap();

        // roundtrip reads and metadata
        let bytes = store.read(&path).unwrap().unwrap();
        assert_eq!(bytes, b"hello");

        let byte_len = store.byte_len(&path).unwrap().unwrap();
        assert_eq!(byte_len, 5);
        assert_eq!(store.read(&path).unwrap().unwrap(), b"hello");
    }
}
