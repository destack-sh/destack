use std::path::{Path, PathBuf};

use destack_core::stable_hash_text_128;

/// Directory name for cached build outputs.
pub const CACHE_DIR_NAME: &str = "cache";
/// Directory name for language cache entries.
pub const LANGUAGE_CACHE_DIR_NAME: &str = "language";
/// Directory name for shared workspace partitions.
pub const WORKSPACES_CACHE_DIR_NAME: &str = "workspaces";
/// Directory name for artifact record store entries.
pub const ARTIFACT_STORE_DIR_NAME: &str = "artifacts";
/// Directory name for content store entries.
pub const CONTENT_STORE_DIR_NAME: &str = "contents";
/// Lock file name for persistent store writes.
pub const STORE_LOCK_FILE_NAME: &str = "store.lock";

/// Filesystem layout for one repository store partition.
#[derive(Debug, Clone)]
pub struct RepositoryStoreLayout {
    /// The `.destack` cache root.
    cache_root: PathBuf,
    /// The stable repository root.
    repository_root: PathBuf,
    /// Whether this root is shared across workspaces.
    is_shared_root: bool,
}

impl RepositoryStoreLayout {
    /// Create one repository store layout.
    pub fn new(cache_root: &Path, repository_root: &Path, is_shared_root: bool) -> Self {
        Self {
            cache_root: cache_root.to_path_buf(),
            repository_root: repository_root.to_path_buf(),
            is_shared_root,
        }
    }

    /// Return the stable workspace cache key.
    fn repository_key(&self) -> String {
        let repository_root = self.repository_root.to_string_lossy();

        format!("{:032x}", stable_hash_text_128(&repository_root))
    }

    /// Return the language cache root.
    fn language_root(&self) -> PathBuf {
        self.cache_root
            .join(CACHE_DIR_NAME)
            .join(LANGUAGE_CACHE_DIR_NAME)
    }

    /// Return the repository store root.
    fn store_root(&self) -> PathBuf {
        if self.is_shared_root {
            return self
                .language_root()
                .join(WORKSPACES_CACHE_DIR_NAME)
                .join(self.repository_key());
        }

        self.language_root()
    }

    /// Return the artifact record root.
    pub(crate) fn artifact_root(&self, partition: &str) -> PathBuf {
        self.store_root()
            .join(ARTIFACT_STORE_DIR_NAME)
            .join(partition)
    }

    /// Return the content root.
    pub fn content_root(&self) -> PathBuf {
        self.store_root().join(CONTENT_STORE_DIR_NAME)
    }

    /// Return the persistent store write lock path.
    pub(crate) fn store_lock_path(&self) -> PathBuf {
        self.store_root().join(STORE_LOCK_FILE_NAME)
    }
}
