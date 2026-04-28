use std::path::Path;

/// Errors produced by cache stores.
#[derive(Debug)]
pub enum CacheStoreError {
    /// The cache store failed to read or write.
    Io(std::io::Error),
    /// The cache entry already exists.
    AlreadyExists,
}

impl std::fmt::Display for CacheStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format cache store errors
        match self {
            CacheStoreError::Io(error) => write!(f, "cache store io error: {error}"),
            CacheStoreError::AlreadyExists => write!(f, "cache entry already exists"),
        }
    }
}

impl std::error::Error for CacheStoreError {}

impl From<std::io::Error> for CacheStoreError {
    fn from(error: std::io::Error) -> Self {
        CacheStoreError::Io(error)
    }
}

/// Interface for cache storage backends.
pub trait CacheStore: std::fmt::Debug + Send + Sync {
    /// Run one operation under the cache lock.
    fn with_exclusive_lock(
        &self,
        path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), CacheStoreError>;

    /// Read cache bytes from a path.
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError>;

    /// Publish cache bytes only when the path does not already exist.
    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError>;

    /// Return the cache entry byte length if present.
    fn byte_len(&self, path: &Path) -> Result<Option<u64>, CacheStoreError>;
}
