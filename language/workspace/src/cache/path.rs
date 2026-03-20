use std::path::{Path, PathBuf};

use crate::{CacheScope, resolve_global_cache_root};

/// Default cache directory name for workspace scoped caches.
pub const DEFAULT_CACHE_DIR: &str = ".destack";
/// Default cache directory name under home when xdg is missing.
pub const DEFAULT_HOME_CACHE_DIR: &str = ".cache";
/// Default cache directory name for global caches.
pub const DEFAULT_GLOBAL_CACHE_DIR: &str = "destack";
/// Namespace for compiler cache entries.
pub const DEFAULT_COMPILER_CACHE_NAMESPACE: &str = "compiler";
/// Directory name for persisted artifact entries.
pub const ARTIFACT_STORE_DIR_NAME: &str = "artifacts";
/// Workspace index file name.
pub const WORKSPACE_INDEX_FILE_NAME: &str = "workspace.bin";
/// Workspace index lock file name.
pub const WORKSPACE_INDEX_LOCK_FILE_NAME: &str = "workspace-index.lock";

/// Resolve a cache root for the provided scope and cache dir.
pub fn resolve_cache_root_for_scope(
    base_dir: &Path,
    cache_dir: Option<&Path>,
    scope: CacheScope,
) -> PathBuf {
    // honor explicit cache directory paths first
    if let Some(cache_dir) = cache_dir {
        if cache_dir.is_absolute() {
            return cache_dir.to_path_buf();
        }

        if scope == CacheScope::Global
            && let Some(global_root) = resolve_global_cache_root(DEFAULT_GLOBAL_CACHE_DIR)
        {
            return global_root.join(cache_dir);
        }

        return base_dir.join(cache_dir);
    }

    // resolve global cache roots when requested
    if scope == CacheScope::Global
        && let Some(global_root) = resolve_global_cache_root(DEFAULT_GLOBAL_CACHE_DIR)
    {
        return global_root;
    }

    base_dir.join(DEFAULT_CACHE_DIR)
}

/// Resolve a path relative to a base directory.
pub fn resolve_cache_dir(path: &Path, base_dir: &Path) -> PathBuf {
    // resolve relative paths against the base directory
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}
