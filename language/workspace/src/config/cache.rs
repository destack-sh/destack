use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::resolve_global_cache_root;

const DEFAULT_CACHE_DIR: &str = ".destack";
const DEFAULT_GLOBAL_CACHE_DIR: &str = "destack";

/// Cache configuration options.
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// Cache mode.
    pub mode: CacheMode,
    /// Cache directory path.
    pub dir: Option<PathBuf>,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
    /// Cache eviction policy.
    pub policy: CachePolicy,
    /// Cache validation strategy.
    pub validate: CacheValidate,
    /// Cache scope selection.
    pub scope: CacheScope,
}

/// Cache mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// Disable caching.
    #[default]
    Off,
    /// Use in-memory caching only.
    Memory,
    /// Use on-disk caching.
    Disk,
}

/// Cache eviction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Least recently used eviction.
    #[default]
    Lru,
    /// Time to live eviction.
    Ttl,
}

/// Cache validation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CacheValidate {
    /// Always validate cache entries strictly.
    #[default]
    Strict,
    /// Validate only on mismatched metadata or changes.
    Fast,
}

/// Cache scope selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheScope {
    /// Cache entries are workspace-local.
    #[default]
    Workspace,
    /// Cache entries are stored in a global shared cache.
    Global,
}

/// Cache options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CacheJson {
    /// Cache mode.
    pub mode: Option<CacheModeJson>,
    /// Cache directory path.
    pub dir: Option<String>,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
    /// Cache eviction policy.
    pub policy: Option<CachePolicyJson>,
    /// Cache validation policy.
    pub validate: Option<CacheValidateJson>,
    /// Cache scope selection.
    pub scope: Option<CacheScopeJson>,
}

impl From<&CacheJson> for CacheOptions {
    fn from(json: &CacheJson) -> Self {
        Self {
            mode: json.mode.map(CacheMode::from).unwrap_or_default(),
            dir: json.dir.as_ref().map(PathBuf::from),
            max_size_mb: json.max_size_mb,
            policy: json.policy.map(CachePolicy::from).unwrap_or_default(),
            validate: json.validate.map(CacheValidate::from).unwrap_or_default(),
            scope: json.scope.map(CacheScope::from).unwrap_or_default(),
        }
    }
}

/// Cache mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheModeJson {
    /// Disable caching.
    Off,
    /// Use in-memory caching only.
    Memory,
    /// Use on-disk caching.
    Disk,
}

impl From<CacheModeJson> for CacheMode {
    fn from(value: CacheModeJson) -> Self {
        match value {
            CacheModeJson::Off => CacheMode::Off,
            CacheModeJson::Memory => CacheMode::Memory,
            CacheModeJson::Disk => CacheMode::Disk,
        }
    }
}

/// Cache eviction policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CachePolicyJson {
    /// Least recently used eviction.
    Lru,
    /// Time to live eviction.
    Ttl,
}

impl From<CachePolicyJson> for CachePolicy {
    fn from(value: CachePolicyJson) -> Self {
        match value {
            CachePolicyJson::Lru => CachePolicy::Lru,
            CachePolicyJson::Ttl => CachePolicy::Ttl,
        }
    }
}

/// Cache validation policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheValidateJson {
    /// Always validate cache entries strictly.
    Strict,
    /// Validate only on mismatched metadata or changes.
    Fast,
}

impl From<CacheValidateJson> for CacheValidate {
    fn from(value: CacheValidateJson) -> Self {
        match value {
            CacheValidateJson::Strict => CacheValidate::Strict,
            CacheValidateJson::Fast => CacheValidate::Fast,
        }
    }
}

/// Cache scope selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CacheScopeJson {
    /// Cache entries are workspace-local.
    Workspace,
    /// Cache entries are stored in a global shared cache.
    Global,
}

impl From<CacheScopeJson> for CacheScope {
    fn from(value: CacheScopeJson) -> Self {
        match value {
            CacheScopeJson::Workspace => CacheScope::Workspace,
            CacheScopeJson::Global => CacheScope::Global,
        }
    }
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
