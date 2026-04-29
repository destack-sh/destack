use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_WORKSPACE_CACHE_DIRECTORY: &str = ".destack";

/// Artifact cache configuration options.
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// The effective artifact cache mode.
    pub mode: CacheMode,
}

/// Artifact cache mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// Disable artifact caching.
    #[default]
    Off,
    /// Use persisted artifact caching.
    Disk,
}

/// Artifact cache options from `destack.json`.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CacheJson {
    /// The configured artifact cache mode.
    pub mode: Option<CacheModeJson>,
}

impl From<&CacheJson> for CacheOptions {
    fn from(json: &CacheJson) -> Self {
        Self {
            mode: json.mode.map(CacheMode::from).unwrap_or_default(),
        }
    }
}

/// Artifact cache mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheModeJson {
    /// Disable artifact caching.
    Off,
    /// Use persisted artifact caching.
    Disk,
}

impl From<CacheModeJson> for CacheMode {
    fn from(value: CacheModeJson) -> Self {
        match value {
            CacheModeJson::Off => CacheMode::Off,
            CacheModeJson::Disk => CacheMode::Disk,
        }
    }
}

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
