use std::path::{Path, PathBuf};

const DEFAULT_WORKSPACE_CACHE_DIRECTORY: &str = ".destack";

/// Resolve one cache root from one workspace root and optional directory override.
pub fn resolve_cache_root(
    workspace_root: &Path,
    cache_directory_override: Option<&Path>,
) -> PathBuf {
    if let Some(cache_directory_override) = cache_directory_override {
        if cache_directory_override.is_absolute() {
            return cache_directory_override.to_path_buf();
        }

        return workspace_root.join(cache_directory_override);
    }

    workspace_root.join(DEFAULT_WORKSPACE_CACHE_DIRECTORY)
}
