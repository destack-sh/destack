use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;

use super::{BlobStore, BlobStoreError};

/// Blob stored in memory.
#[derive(Debug, Clone)]
struct MemoryBlobEntry {
    /// Stored bytes.
    bytes: Vec<u8>,
}

/// Blob store backed by memory.
#[derive(Debug, Default)]
pub struct MemoryBlobStore {
    /// Stored blobs keyed by path.
    entries: RwLock<HashMap<PathBuf, MemoryBlobEntry>>,
    /// Lock guard for shared and exclusive blob access.
    lock: RwLock<()>,
}

impl MemoryBlobStore {
    /// Create a memory blob store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlobStore for MemoryBlobStore {
    fn with_exclusive_lock(
        &self,
        _path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), BlobStoreError> {
        let _lock = self.lock.write();

        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError> {
        let entries = self.entries.read();
        Ok(entries.get(path).map(|entry| entry.bytes.clone()))
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), BlobStoreError> {
        let entry = MemoryBlobEntry {
            bytes: bytes.to_vec(),
        };
        let mut entries = self.entries.write();
        if entries.contains_key(path) {
            return Err(BlobStoreError::AlreadyExists);
        }

        entries.insert(path.to_path_buf(), entry);
        Ok(())
    }

    fn byte_len(&self, path: &Path) -> Result<Option<u64>, BlobStoreError> {
        let entries = self.entries.read();
        let entry = match entries.get(path) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        Ok(Some(entry.bytes.len() as u64))
    }

    fn entries(&self, root: &Path) -> Result<Vec<PathBuf>, BlobStoreError> {
        let entries = self.entries.read();
        let paths = entries
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect();

        Ok(paths)
    }

    fn remove(&self, path: &Path) -> Result<(), BlobStoreError> {
        let mut entries = self.entries.write();
        entries.remove(path);

        Ok(())
    }
}
