use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;

use super::{CacheLock, CacheMetadata, CacheStore, CacheStoreError, CacheStoreKind};

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
    fn kind(&self) -> CacheStoreKind {
        CacheStoreKind::Memory
    }

    fn lock_shared(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Ok(CacheLock::InProcessRead(self.lock.read()))
    }

    fn lock_exclusive(&self, _path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
        Ok(CacheLock::InProcessWrite(self.lock.write()))
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        let entries = self.entries.read();
        Ok(entries.get(path).map(|entry| entry.bytes.clone()))
    }

    fn read_prefix(&self, path: &Path, limit: usize) -> Result<Option<Vec<u8>>, CacheStoreError> {
        let entries = self.entries.read();
        let bytes = entries
            .get(path)
            .map(|entry| entry.bytes.iter().take(limit).copied().collect());

        Ok(bytes)
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
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

        let _lock = store.lock_exclusive(&path).unwrap();
        store.write_atomic(&path, b"hello").unwrap();

        let bytes = store.read(&path).unwrap().unwrap();
        assert_eq!(bytes, b"hello");

        let metadata = store.metadata(&path).unwrap().unwrap();
        assert_eq!(metadata.size_bytes, 5);
        assert!(store.exists(&path).unwrap());

        store.remove(&path).unwrap();
        assert!(!store.exists(&path).unwrap());
        assert!(store.read(&path).unwrap().is_none());
    }
}
