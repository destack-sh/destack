use std::io::ErrorKind;
use std::path::Path;

use super::{BlobStore, BlobStoreError};

/// Disk blob store unavailable on unsupported targets.
#[derive(Debug, Default, Clone)]
pub struct DiskBlobStore;

impl DiskBlobStore {
    /// Create a disk blob store.
    pub fn new() -> Self {
        Self
    }
}

impl BlobStore for DiskBlobStore {
    fn with_exclusive_lock(
        &self,
        _path: &Path,
        _operation: &mut dyn FnMut(),
    ) -> Result<(), BlobStoreError> {
        Err(unsupported())
    }

    fn read(&self, _path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError> {
        Err(unsupported())
    }

    fn write_once(&self, _path: &Path, _bytes: &[u8]) -> Result<(), BlobStoreError> {
        Err(unsupported())
    }

    fn byte_len(&self, _path: &Path) -> Result<Option<u64>, BlobStoreError> {
        Err(unsupported())
    }

    fn entries(&self, _root: &Path) -> Result<Vec<std::path::PathBuf>, BlobStoreError> {
        Err(unsupported())
    }

    fn remove(&self, _path: &Path) -> Result<(), BlobStoreError> {
        Err(unsupported())
    }
}

/// Build one unsupported blob-store error.
fn unsupported() -> BlobStoreError {
    BlobStoreError::from(std::io::Error::new(
        ErrorKind::Unsupported,
        "disk blob store is not supported on this target",
    ))
}
