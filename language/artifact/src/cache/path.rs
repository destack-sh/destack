use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rustc_hash::FxHasher;

/// Directory name for persisted cache entries.
pub const CACHE_DIR_NAME: &str = "cache";
/// Directory name for language cache entries.
pub const LANGUAGE_CACHE_DIR_NAME: &str = "language";
/// Directory name for shared workspace partitions.
pub const WORKSPACES_CACHE_DIR_NAME: &str = "workspaces";
/// Directory name for persisted artifact image entries.
pub const ARTIFACT_CACHE_DIR_NAME: &str = "artifacts";
/// Directory name for persisted artifact content entries.
pub const ARTIFACT_CONTENTS_DIR_NAME: &str = "contents";
/// Directory name for current artifact content ids.
pub const ARTIFACT_CACHE_CURRENT_DIR_NAME: &str = "current";
/// Lock file name for current artifact updates.
pub const ARTIFACT_CACHE_LOCK_FILE_NAME: &str = "artifacts.lock";

/// One persisted language cache layout.
#[derive(Debug, Clone)]
pub struct LanguageCacheLayout {
    /// The `.destack` cache root.
    cache_root: PathBuf,
    /// The stable workspace root.
    workspace_root: PathBuf,
    /// The persisted cache abi.
    cache_abi: String,
    /// Whether this root is shared across workspaces.
    is_shared_root: bool,
}

impl LanguageCacheLayout {
    /// Create a persisted language cache layout.
    pub fn new(
        cache_root: &Path,
        workspace_root: &Path,
        cache_abi: impl Into<String>,
        is_shared_root: bool,
    ) -> Self {
        Self {
            cache_root: cache_root.to_path_buf(),
            workspace_root: workspace_root.to_path_buf(),
            cache_abi: cache_abi.into(),
            is_shared_root,
        }
    }

    /// Return the language cache abi.
    fn cache_abi(&self) -> &str {
        &self.cache_abi
    }

    /// Return the stable workspace cache key.
    fn workspace_key(&self) -> String {
        let mut hasher = FxHasher::default();
        self.workspace_root.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Return the language cache root.
    fn language_root(&self) -> PathBuf {
        self.cache_root
            .join(CACHE_DIR_NAME)
            .join(LANGUAGE_CACHE_DIR_NAME)
    }

    /// Return the cache abi root.
    fn abi_root(&self) -> PathBuf {
        self.language_root().join(self.cache_abi())
    }

    /// Return the workspace cache root.
    fn workspace_root(&self) -> PathBuf {
        if self.is_shared_root {
            return self
                .abi_root()
                .join(WORKSPACES_CACHE_DIR_NAME)
                .join(self.workspace_key());
        }

        self.abi_root()
    }

    /// Return the persisted artifact cache root.
    fn artifact_root(&self) -> PathBuf {
        self.workspace_root().join(ARTIFACT_CACHE_DIR_NAME)
    }

    /// Return the persisted artifact content root.
    pub fn content_root(&self) -> PathBuf {
        self.artifact_root().join(ARTIFACT_CONTENTS_DIR_NAME)
    }

    /// Return the current artifact root.
    pub(crate) fn current_root(&self) -> PathBuf {
        self.artifact_root().join(ARTIFACT_CACHE_CURRENT_DIR_NAME)
    }

    /// Return the artifact cache lock path.
    pub(crate) fn artifact_lock_path(&self) -> PathBuf {
        self.artifact_root().join(ARTIFACT_CACHE_LOCK_FILE_NAME)
    }
}
