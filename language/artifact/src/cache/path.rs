use std::path::{Path, PathBuf};

use destack_core::stable_hash_text_128;

/// Directory name for cache entries.
pub const CACHE_DIR_NAME: &str = "cache";
/// Directory name for language cache entries.
pub const LANGUAGE_CACHE_DIR_NAME: &str = "language";
/// Directory name for shared workspace partitions.
pub const WORKSPACES_CACHE_DIR_NAME: &str = "workspaces";
/// Directory name for artifact image entries.
pub const ARTIFACT_CACHE_DIR_NAME: &str = "artifacts";
/// Directory name for artifact images.
pub const ARTIFACT_IMAGES_DIR_NAME: &str = "images";
/// Lock file name for artifact image writes.
pub const ARTIFACT_IMAGE_LOCK_FILE_NAME: &str = "images.lock";

/// One artifact image cache layout.
#[derive(Debug, Clone)]
pub struct ArtifactImageCacheLayout {
    /// The `.destack` cache root.
    cache_root: PathBuf,
    /// The stable workspace root.
    workspace_root: PathBuf,
    /// The cache ABI.
    cache_abi: String,
    /// Whether this root is shared across workspaces.
    is_shared_root: bool,
}

impl ArtifactImageCacheLayout {
    /// Create an artifact image cache layout.
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

    /// Return the language cache ABI.
    fn cache_abi(&self) -> &str {
        &self.cache_abi
    }

    /// Return the stable workspace cache key.
    fn workspace_key(&self) -> String {
        let workspace_root = self.workspace_root.to_string_lossy();

        format!("{:032x}", stable_hash_text_128(&workspace_root))
    }

    /// Return the language cache root.
    fn language_root(&self) -> PathBuf {
        self.cache_root
            .join(CACHE_DIR_NAME)
            .join(LANGUAGE_CACHE_DIR_NAME)
    }

    /// Return the cache ABI root.
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

    /// Return the artifact cache root.
    fn artifact_root(&self) -> PathBuf {
        self.workspace_root().join(ARTIFACT_CACHE_DIR_NAME)
    }

    /// Return the artifact image root.
    pub fn image_root(&self) -> PathBuf {
        self.artifact_root().join(ARTIFACT_IMAGES_DIR_NAME)
    }

    /// Return the artifact image write lock path.
    pub(crate) fn image_lock_path(&self) -> PathBuf {
        self.artifact_root().join(ARTIFACT_IMAGE_LOCK_FILE_NAME)
    }
}
