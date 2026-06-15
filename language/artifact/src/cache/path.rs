use std::path::{Path, PathBuf};

use destack_core::stable_hash_text_128;

/// Directory name for cache entries.
pub const CACHE_DIR_NAME: &str = "cache";
/// Directory name for language cache entries.
pub const LANGUAGE_CACHE_DIR_NAME: &str = "language";
/// Directory name for shared workspace partitions.
pub const WORKSPACES_CACHE_DIR_NAME: &str = "workspaces";
/// Directory name for artifact record entries.
pub const ARTIFACT_CACHE_DIR_NAME: &str = "artifacts";
/// Directory name for content cache entries.
pub const CONTENT_CACHE_DIR_NAME: &str = "contents";
/// Lock file name for persistent cache writes.
pub const CACHE_LOCK_FILE_NAME: &str = "cache.lock";

/// Filesystem layout for one repository cache partition.
#[derive(Debug, Clone)]
pub struct RepositoryCacheLayout {
    /// The `.destack` cache root.
    cache_root: PathBuf,
    /// The stable repository root.
    repository_root: PathBuf,
    /// Whether this root is shared across workspaces.
    is_shared_root: bool,
}

impl RepositoryCacheLayout {
    /// Create one repository cache layout.
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

    /// Return the repository cache root.
    fn repository_root(&self) -> PathBuf {
        if self.is_shared_root {
            return self
                .language_root()
                .join(WORKSPACES_CACHE_DIR_NAME)
                .join(self.repository_key());
        }

        self.language_root()
    }

    /// Return the artifact cache root.
    pub(crate) fn artifact_root(&self, build_fingerprint: &str) -> PathBuf {
        self.repository_root()
            .join(ARTIFACT_CACHE_DIR_NAME)
            .join(build_fingerprint)
    }

    /// Return the content cache root.
    pub fn content_root(&self) -> PathBuf {
        self.repository_root().join(CONTENT_CACHE_DIR_NAME)
    }

    /// Return the persistent cache write lock path.
    pub(crate) fn cache_lock_path(&self) -> PathBuf {
        self.repository_root().join(CACHE_LOCK_FILE_NAME)
    }
}
