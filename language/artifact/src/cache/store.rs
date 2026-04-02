use std::path::{Path, PathBuf};

/// Metadata for cache entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheMetadata {
    /// Size of the cached entry in bytes.
    pub size_bytes: u64,
    /// Last modification time in nanoseconds since unix epoch.
    pub modified_ns: Option<u64>,
}

/// Errors produced by cache stores.
#[derive(Debug)]
pub enum CacheStoreError {
    /// The cache store failed to read or write.
    Io(std::io::Error),
}

impl std::fmt::Display for CacheStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format cache store errors
        match self {
            CacheStoreError::Io(error) => write!(f, "cache store io error: {error}"),
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
    fn with_lock(&self, path: &Path, operation: &mut dyn FnMut()) -> Result<(), CacheStoreError>;

    /// Read cache bytes from a path.
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError>;

    /// Write cache bytes using an atomic replace.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError>;

    /// Touch a cache entry for access tracking.
    fn touch(&self, path: &Path) -> Result<(), CacheStoreError>;

    /// Check whether a cache entry exists at the path.
    fn exists(&self, path: &Path) -> Result<bool, CacheStoreError>;

    /// List all cache entries rooted under one path.
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, CacheStoreError>;

    /// Remove a cache entry if present.
    fn remove(&self, path: &Path) -> Result<(), CacheStoreError>;

    /// Return metadata for a cache entry if present.
    fn metadata(&self, path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError>;
}
