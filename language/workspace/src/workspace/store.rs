use std::path::{Path, PathBuf};

use crate::{
    CacheStore, CacheStoreError, DEFAULT_COMPILER_CACHE_NAMESPACE, WORKSPACE_INDEX_FILE_NAME,
    WORKSPACE_INDEX_LOCK_FILE_NAME,
};

use super::{WorkspaceIndexError, WorkspaceIndexHeader, WorkspaceIndexSnapshot};

/// Maximum size allowed for workspace index payloads.
pub const WORKSPACE_INDEX_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
/// Cache backed workspace index reader and writer.
#[derive(Debug)]
pub struct WorkspaceStore<'a> {
    /// The cache store backing the index.
    store: &'a dyn CacheStore,
    /// Path to the workspace index payload.
    index_path: PathBuf,
    /// Path to the workspace index lock.
    lock_path: PathBuf,
}

impl<'a> WorkspaceStore<'a> {
    /// Create a workspace index store for a cache root.
    pub fn new(store: &'a dyn CacheStore, cache_root: &Path) -> Self {
        let base = cache_root.join(DEFAULT_COMPILER_CACHE_NAMESPACE);
        let index_path = base.join(WORKSPACE_INDEX_FILE_NAME);
        let lock_path = base.join(WORKSPACE_INDEX_LOCK_FILE_NAME);
        Self {
            store,
            index_path,
            lock_path,
        }
    }

    /// Load a workspace index with shared locking.
    pub fn load(
        &self,
        expected: &WorkspaceIndexHeader,
    ) -> Result<Option<WorkspaceIndexSnapshot>, WorkspaceIndexError> {
        self.store
            .with_shared_lock(&self.lock_path, || {
                self.read_unlocked(expected).map_err(CacheStoreError::from)
            })
            .map_err(WorkspaceIndexError::from)
    }

    /// Save a workspace index with exclusive locking.
    pub fn save(&self, snapshot: &WorkspaceIndexSnapshot) -> Result<(), WorkspaceIndexError> {
        self.store
            .with_exclusive_lock(&self.lock_path, || {
                self.write_unlocked(snapshot).map_err(CacheStoreError::from)
            })
            .map_err(WorkspaceIndexError::from)
    }

    /// Read a workspace index without locking.
    fn read_unlocked(
        &self,
        expected: &WorkspaceIndexHeader,
    ) -> Result<Option<WorkspaceIndexSnapshot>, WorkspaceIndexError> {
        // guard against oversized payloads before reading
        if let Some(metadata) = self.store.metadata(&self.index_path)?
            && metadata.size_bytes > WORKSPACE_INDEX_LIMIT_BYTES
        {
            return Err(WorkspaceIndexError::SizeLimitExceeded {
                limit: WORKSPACE_INDEX_LIMIT_BYTES,
                actual: metadata.size_bytes,
            });
        }

        // read the index bytes
        let Some(bytes) = self.store.read(&self.index_path)? else {
            return Ok(None);
        };
        let size = bytes.len() as u64;
        if size > WORKSPACE_INDEX_LIMIT_BYTES {
            return Err(WorkspaceIndexError::SizeLimitExceeded {
                limit: WORKSPACE_INDEX_LIMIT_BYTES,
                actual: size,
            });
        }

        // decode the index
        let snapshot: WorkspaceIndexSnapshot =
            postcard::from_bytes(&bytes).map_err(WorkspaceIndexError::Deserialize)?;

        // validate the header
        if !snapshot.header.matches(expected) {
            return Ok(None);
        }

        Ok(Some(snapshot))
    }

    /// Write a workspace index without locking.
    fn write_unlocked(&self, snapshot: &WorkspaceIndexSnapshot) -> Result<(), WorkspaceIndexError> {
        // serialize the index
        let bytes = postcard::to_allocvec(snapshot).map_err(WorkspaceIndexError::Serialize)?;

        // guard against oversized payloads
        let size = bytes.len() as u64;
        if size > WORKSPACE_INDEX_LIMIT_BYTES {
            return Err(WorkspaceIndexError::SizeLimitExceeded {
                limit: WORKSPACE_INDEX_LIMIT_BYTES,
                actual: size,
            });
        }

        // write atomically
        self.store.write_atomic(&self.index_path, &bytes)?;

        Ok(())
    }
}

impl From<CacheStoreError> for WorkspaceIndexError {
    fn from(error: CacheStoreError) -> Self {
        match error {
            CacheStoreError::Io(error) => Self::Io(error),
        }
    }
}

impl From<WorkspaceIndexError> for CacheStoreError {
    fn from(error: WorkspaceIndexError) -> Self {
        Self::Io(std::io::Error::other(error))
    }
}
