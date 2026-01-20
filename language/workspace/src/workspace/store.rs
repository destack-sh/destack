use std::path::{Path, PathBuf};

use crate::{CacheStore, DEFAULT_COMPILER_CACHE_NAMESPACE, WORKSPACE_INDEX_FILE_NAME};

use super::{WorkspaceIndexError, WorkspaceIndexHeader, WorkspaceIndexSnapshot};

/// Maximum size allowed for workspace index payloads.
pub const WORKSPACE_INDEX_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
/// Workspace index lock file name.
pub const WORKSPACE_INDEX_LOCK_FILE_NAME: &str = "workspace.lock";

/// Cache backed workspace index reader and writer.
#[derive(Debug)]
pub struct WorkspaceIndexStore<'a> {
    /// The cache store backing the index.
    store: &'a dyn CacheStore,
    /// Path to the workspace index payload.
    index_path: PathBuf,
    /// Path to the workspace index lock.
    lock_path: PathBuf,
}

impl<'a> WorkspaceIndexStore<'a> {
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

    /// Load a workspace index snapshot with shared locking.
    pub fn load(
        &self,
        expected: &WorkspaceIndexHeader,
    ) -> Result<Option<WorkspaceIndexSnapshot>, WorkspaceIndexError> {
        let _lock = self.store.lock_shared(&self.lock_path)?;
        self.read_unlocked(expected)
    }

    /// Save a workspace index snapshot with exclusive locking.
    pub fn save(&self, snapshot: &WorkspaceIndexSnapshot) -> Result<(), WorkspaceIndexError> {
        let _lock = self.store.lock_exclusive(&self.lock_path)?;
        self.write_unlocked(snapshot)
    }

    /// Read a workspace index snapshot without locking.
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

        // decode the snapshot
        let snapshot: WorkspaceIndexSnapshot =
            postcard::from_bytes(&bytes).map_err(WorkspaceIndexError::Deserialize)?;

        // validate the header
        if !snapshot.header.matches(expected) {
            return Ok(None);
        }

        Ok(Some(snapshot))
    }

    /// Write a workspace index snapshot without locking.
    fn write_unlocked(&self, snapshot: &WorkspaceIndexSnapshot) -> Result<(), WorkspaceIndexError> {
        // serialize the snapshot
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
