use std::path::Path;

/// Errors produced by blob stores.
#[derive(Debug)]
pub enum BlobStoreError {
    /// The blob store failed to read or write.
    Io(Box<std::io::Error>),
    /// The blob already exists.
    AlreadyExists,
}

impl std::fmt::Display for BlobStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format blob store errors
        match self {
            BlobStoreError::Io(error) => write!(f, "blob store io error: {error}"),
            BlobStoreError::AlreadyExists => write!(f, "blob already exists"),
        }
    }
}

impl std::error::Error for BlobStoreError {}

impl From<std::io::Error> for BlobStoreError {
    fn from(error: std::io::Error) -> Self {
        BlobStoreError::Io(Box::new(error))
    }
}

/// Interface for opaque byte storage backends.
pub trait BlobStore: std::fmt::Debug + Send + Sync {
    /// Run one operation under the storage lock.
    fn with_exclusive_lock(
        &self,
        path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), BlobStoreError>;

    /// Read bytes from a path.
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError>;

    /// Publish bytes only when the path does not already exist.
    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), BlobStoreError>;

    /// Return the byte length if present.
    fn byte_len(&self, path: &Path) -> Result<Option<u64>, BlobStoreError>;

    /// List entry paths below one root.
    fn entries(&self, root: &Path) -> Result<Vec<std::path::PathBuf>, BlobStoreError>;

    /// Remove one blob if it exists.
    fn remove(&self, path: &Path) -> Result<(), BlobStoreError>;
}
