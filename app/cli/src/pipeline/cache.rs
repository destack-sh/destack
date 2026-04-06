use std::path::{Path, PathBuf};

use destack_workspace::resolve_cache_root;

use crate::common::ProgramArgs;

/// Source of one resolved cache location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// Cache directory from one CLI override.
    Override,
    /// Default cache directory location.
    Default,
}

/// Resolved cache location.
#[derive(Debug, Clone)]
pub struct CacheLocation {
    /// The resolved cache directory.
    pub directory: PathBuf,
    /// The origin of the cache path.
    pub source: CacheSource,
}

/// Resolve one cache directory using runtime overrides and defaults.
pub fn resolve_cache_location(
    program_args: &ProgramArgs,
    workspace_root: &Path,
    cwd: &Path,
) -> CacheLocation {
    // honor cli overrides first
    if let Some(cache_directory) = program_args.cache_dir.as_ref() {
        let directory = resolve_path(cache_directory, cwd);
        return CacheLocation {
            directory,
            source: CacheSource::Override,
        };
    }

    // default to the workspace root
    CacheLocation {
        directory: resolve_cache_root(workspace_root, None),
        source: CacheSource::Default,
    }
}

/// Resolve one path relative to one root.
fn resolve_path(path: &Path, root: &Path) -> PathBuf {
    // resolve relative paths against the root
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
