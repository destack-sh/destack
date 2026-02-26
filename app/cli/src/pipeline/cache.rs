use std::path::{Path, PathBuf};

use destack_workspace::{CacheScope, DsConfig, resolve_cache_root_for_scope};

use crate::common::ProgramArgs;

/// Source of a resolved cache location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// Cache directory from CLI override.
    Override,
    /// Cache directory from dsconfig.json.
    DsConfig,
    /// Default cache directory location.
    Default,
}

/// Resolved cache location.
#[derive(Debug, Clone)]
pub struct CacheLocation {
    /// The resolved cache directory.
    pub dir: PathBuf,
    /// The origin of the cache path.
    pub source: CacheSource,
}

/// Resolve the cache directory using overrides and dsconfig settings.
pub fn resolve_cache_location(
    program_args: &ProgramArgs,
    dsconfig: Option<&DsConfig>,
    workspace_root: &Path,
    cwd: &Path,
) -> CacheLocation {
    // honor cli overrides first
    if let Some(cache_dir) = program_args.cache_dir.as_ref() {
        let dir = resolve_path(cache_dir, cwd);
        return CacheLocation {
            dir,
            source: CacheSource::Override,
        };
    }

    // fall back to dsconfig cache settings
    if let Some(dsconfig) = dsconfig {
        let dir = resolve_cache_root_for_scope(
            &dsconfig.directory,
            dsconfig.options.cache.dir.as_deref(),
            dsconfig.options.cache.scope,
        );
        return CacheLocation {
            dir,
            source: CacheSource::DsConfig,
        };
    }

    // default to workspace root
    CacheLocation {
        dir: resolve_cache_root_for_scope(workspace_root, None, CacheScope::Workspace),
        source: CacheSource::Default,
    }
}

/// Resolve a path relative to the provided root.
fn resolve_path(path: &Path, root: &Path) -> PathBuf {
    // resolve relative paths against the root
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
