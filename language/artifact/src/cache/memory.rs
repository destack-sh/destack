use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;

use super::{CacheMetadata, CacheStore, CacheStoreError};

/// Cache entry stored in memory.
#[derive(Debug, Clone)]
struct MemoryCacheEntry {
    /// Cached bytes.
    bytes: Vec<u8>,
    /// Last modification time in nanoseconds since unix epoch.
    modified_ns: Option<u64>,
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
    fn with_lock(&self, _path: &Path, operation: &mut dyn FnMut()) -> Result<(), CacheStoreError> {
        let _lock = self.lock.write();

        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        let entries = self.entries.read();
        Ok(entries.get(path).map(|entry| entry.bytes.clone()))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
        let entry = MemoryCacheEntry {
            bytes: bytes.to_vec(),
            modified_ns: system_time_to_nanos(SystemTime::now()),
        };
        let mut entries = self.entries.write();
        entries.insert(path.to_path_buf(), entry);
        Ok(())
    }

    fn touch(&self, path: &Path) -> Result<(), CacheStoreError> {
        let mut entries = self.entries.write();
        if let Some(entry) = entries.get_mut(path) {
            entry.modified_ns = system_time_to_nanos(SystemTime::now());
        }
        Ok(())
    }

    fn exists(&self, path: &Path) -> Result<bool, CacheStoreError> {
        let entries = self.entries.read();
        Ok(entries.contains_key(path))
    }

    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, CacheStoreError> {
        let entries = self.entries.read();
        let mut paths = Vec::new();

        // collect all descendant entries
        for entry_path in entries.keys() {
            if entry_path.starts_with(path) {
                paths.push(entry_path.clone());
            }
        }

        Ok(paths)
    }

    fn remove(&self, path: &Path) -> Result<(), CacheStoreError> {
        let mut entries = self.entries.write();
        entries.remove(path);
        Ok(())
    }

    fn metadata(&self, path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        let entries = self.entries.read();
        let entry = match entries.get(path) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        Ok(Some(CacheMetadata {
            size_bytes: entry.bytes.len() as u64,
            modified_ns: entry.modified_ns,
        }))
    }
}

/// Convert a system time into nanoseconds since unix epoch.
fn system_time_to_nanos(time: SystemTime) -> Option<u64> {
    // compute nanoseconds since unix epoch
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos() as u64;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos))
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
            .with_lock(&path, &mut || {
                store.write(&path, b"hello").unwrap();
            })
            .unwrap();

        // roundtrip reads and metadata
        let bytes = store.read(&path).unwrap().unwrap();
        assert_eq!(bytes, b"hello");

        let metadata = store.metadata(&path).unwrap().unwrap();
        assert_eq!(metadata.size_bytes, 5);
        assert!(store.exists(&path).unwrap());

        // removal
        store.remove(&path).unwrap();
        assert!(!store.exists(&path).unwrap());
        assert!(store.read(&path).unwrap().is_none());
    }
}
