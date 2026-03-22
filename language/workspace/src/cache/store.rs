use std::path::Path;

use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

/// Supported cache store kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStoreKind {
    /// Cache store backed by disk.
    Disk,
    /// Cache store backed by memory.
    Memory,
    /// Cache store that performs no work.
    Null,
}

/// Metadata for cache entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheMetadata {
    /// Size of the cached entry in bytes.
    pub size_bytes: u64,
    /// Last modification time in nanoseconds since unix epoch.
    pub modified_ns: Option<u64>,
}

/// Lock guard for cache operations.
#[derive(Debug)]
pub enum CacheLock<'a> {
    /// File lock for disk caches.
    File(std::fs::File),
    /// In process read lock for memory caches.
    InProcessRead(RwLockReadGuard<'a, ()>),
    /// In process write lock for memory caches.
    InProcessWrite(RwLockWriteGuard<'a, ()>),
    /// Noop lock for disabled caches.
    Noop,
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
    /// Return the cache store kind.
    fn kind(&self) -> CacheStoreKind;

    /// Acquire a shared lock for a cache path.
    fn lock_shared(&self, path: &Path) -> Result<CacheLock<'_>, CacheStoreError>;

    /// Acquire an exclusive lock for a cache path.
    fn lock_exclusive(&self, path: &Path) -> Result<CacheLock<'_>, CacheStoreError>;

    /// Read cache bytes from a path.
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError>;

    /// Read one cache byte prefix from a path.
    fn read_prefix(&self, path: &Path, limit: usize) -> Result<Option<Vec<u8>>, CacheStoreError>;

    /// Write cache bytes using an atomic replace.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError>;

    /// Touch a cache entry for access tracking.
    fn touch(&self, path: &Path) -> Result<(), CacheStoreError>;

    /// Check whether a cache entry exists at the path.
    fn exists(&self, path: &Path) -> Result<bool, CacheStoreError>;

    /// Remove a cache entry if present.
    fn remove(&self, path: &Path) -> Result<(), CacheStoreError>;

    /// Return metadata for a cache entry if present.
    fn metadata(&self, path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError>;
}

impl dyn CacheStore + '_ {
    /// Run one operation under a shared cache lock.
    pub fn with_shared_lock<T, E, F>(&self, path: &Path, operation: F) -> Result<T, E>
    where
        E: From<CacheStoreError>,
        F: FnOnce() -> Result<T, E>,
    {
        let _lock = self.lock_shared(path).map_err(E::from)?;
        operation()
    }

    /// Run one operation under an exclusive cache lock.
    pub fn with_exclusive_lock<T, E, F>(&self, path: &Path, operation: F) -> Result<T, E>
    where
        E: From<CacheStoreError>,
        F: FnOnce() -> Result<T, E>,
    {
        let _lock = self.lock_exclusive(path).map_err(E::from)?;
        operation()
    }
}
