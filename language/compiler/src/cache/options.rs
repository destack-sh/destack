use std::path::PathBuf;

use destack_workspace::{CacheMode, CachePolicy, CacheScope, CacheValidate};

/// Cache options for a module.
#[derive(Debug, Clone)]
pub struct CacheOptions {
    /// Cache mode.
    pub mode: CacheMode,
    /// Cache directory path.
    pub dir: PathBuf,
    /// Cache eviction policy.
    pub policy: CachePolicy,
    /// Cache validation strategy.
    pub validate: CacheValidate,
    /// Cache scope selection.
    pub scope: CacheScope,
    /// Maximum cache size in megabytes.
    pub max_size_mb: Option<u64>,
}

impl CacheOptions {
    /// Check whether cache is enabled for this module.
    pub fn is_enabled(&self) -> bool {
        self.mode != CacheMode::Off
    }

    /// Check whether disk caching is enabled for this module.
    pub fn is_disk_enabled(&self) -> bool {
        self.mode == CacheMode::Disk
    }
}
