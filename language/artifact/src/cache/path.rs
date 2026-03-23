use std::path::{Path, PathBuf};

/// Cache scope selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheScope {
    /// Cache entries are workspace-local.
    #[default]
    Workspace,
    /// Cache entries are stored in a global shared cache.
    Global,
}

/// Default cache directory name for workspace scoped caches.
pub const DEFAULT_CACHE_DIR: &str = ".destack";
/// Default cache directory name under home when xdg is missing.
pub const DEFAULT_HOME_CACHE_DIR: &str = ".cache";
/// Default cache directory name for global caches.
pub const DEFAULT_GLOBAL_CACHE_DIR: &str = "destack";
/// Namespace for language cache entries.
pub const DEFAULT_LANGUAGE_CACHE_NAMESPACE: &str = "language";
/// Directory name for persisted compiler cache entries.
pub const DEFAULT_LANGUAGE_CACHE_DIR_NAME: &str = "cache";
/// Workspace index file name.
pub const WORKSPACE_INDEX_FILE_NAME: &str = "workspace-index.bin";
/// Workspace index lock file name.
pub const WORKSPACE_INDEX_LOCK_FILE_NAME: &str = "workspace-index.lock";
/// Explicit destack cache env override.
pub const DESTACK_CACHE_DIR: &str = "DESTACK_CACHE_DIR";
/// XDG cache home env var.
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
/// Unix home env var.
pub const HOME: &str = "HOME";
/// Windows local app data env var.
pub const LOCAL_APPDATA: &str = "LOCAL_APPDATA";
/// Windows user profile env var.
pub const USERPROFILE: &str = "USERPROFILE";

/// Return an explicit cache directory from the environment.
pub fn cache_dir_from_env() -> Option<PathBuf> {
    let value = std::env::var_os(DESTACK_CACHE_DIR)?;
    if value.is_empty() {
        return None;
    }

    Some(PathBuf::from(value))
}

/// Resolve a global cache root directory for the given name.
pub fn resolve_global_cache_root(dir_name: &str) -> Option<PathBuf> {
    if let Some(dir) = cache_dir_from_env() {
        return Some(dir);
    }

    if let Some(xdg) = std::env::var_os(XDG_CACHE_HOME) {
        return Some(PathBuf::from(xdg).join(dir_name));
    }

    if let Some(home) = std::env::var_os(HOME) {
        return Some(
            PathBuf::from(home)
                .join(DEFAULT_HOME_CACHE_DIR)
                .join(dir_name),
        );
    }

    if let Some(local) = std::env::var_os(LOCAL_APPDATA) {
        return Some(PathBuf::from(local).join(dir_name));
    }

    if let Some(profile) = std::env::var_os(USERPROFILE) {
        return Some(
            PathBuf::from(profile)
                .join(DEFAULT_HOME_CACHE_DIR)
                .join(dir_name),
        );
    }

    None
}

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
