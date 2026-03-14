use serde::{Deserialize, Serialize};

/// Runtime heap size-class configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeapSizeClasses {
    /// The built-in default size-class table.
    Default,
    /// One named built-in size-class table.
    Named(String),
    /// One explicit size-class table in bytes.
    Explicit(Vec<usize>),
}

impl Default for HeapSizeClasses {
    fn default() -> Self {
        Self::Default
    }
}

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapOptions {
    /// Growth target percentage.
    pub growth_percent: u32,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// The configured size-class table for small allocations.
    pub size_classes: HeapSizeClasses,
    /// The byte width for managed runs.
    pub managed_run_bytes: usize,
    /// The byte width for raw runs.
    pub raw_run_bytes: usize,
    /// The byte width for chunk-backed extent and shared leaves.
    pub chunk_bytes: usize,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
    /// Hard limit for retained shared-memory bytes.
    pub max_shared_bytes: Option<u64>,
}

impl Default for HeapOptions {
    fn default() -> Self {
        Self {
            growth_percent: 100,
            soft_limit_bytes: None,
            initial_bytes: None,
            size_classes: HeapSizeClasses::Default,
            managed_run_bytes: 16 * 1024,
            raw_run_bytes: 16 * 1024,
            chunk_bytes: 4 * 1024,
            max_bytes: None,
            max_managed_bytes: None,
            max_raw_bytes: None,
            max_shared_bytes: None,
        }
    }
}

/// Runtime heap size-class configuration for JSON deserialization.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum HeapSizeClassesJson {
    /// One named built-in size-class table.
    Named(String),
    /// One explicit size-class table.
    Explicit(Vec<usize>),
}

impl HeapSizeClassesJson {
    /// Convert this JSON configuration into runtime options.
    pub fn to_options(&self) -> HeapSizeClasses {
        match self {
            Self::Named(name) if name == "default" => HeapSizeClasses::Default,
            Self::Named(name) => HeapSizeClasses::Named(name.clone()),
            Self::Explicit(classes) => HeapSizeClasses::Explicit(classes.clone()),
        }
    }
}

/// Runtime heap configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapOptionsJson {
    /// Growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// The configured size-class table for small allocations.
    pub size_classes: Option<HeapSizeClassesJson>,
    /// The byte width for managed runs.
    pub managed_run_bytes: Option<usize>,
    /// The byte width for raw runs.
    pub raw_run_bytes: Option<usize>,
    /// The byte width for chunk-backed extent and shared leaves.
    pub chunk_bytes: Option<usize>,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
    /// Hard limit for retained shared-memory bytes.
    pub max_shared_bytes: Option<u64>,
}

impl HeapOptionsJson {
    /// Apply heap overrides to a base set of options.
    pub fn apply_to(&self, options: &mut HeapOptions) {
        // apply pacing overrides
        if let Some(growth_percent) = self.growth_percent {
            options.growth_percent = growth_percent;
        }

        // apply memory limit overrides
        if let Some(soft_limit_bytes) = self.soft_limit_bytes {
            options.soft_limit_bytes = Some(soft_limit_bytes);
        }
        if let Some(initial_bytes) = self.initial_bytes {
            options.initial_bytes = Some(initial_bytes);
        }
        if let Some(size_classes) = &self.size_classes {
            options.size_classes = size_classes.to_options();
        }
        if let Some(managed_run_bytes) = self.managed_run_bytes {
            options.managed_run_bytes = managed_run_bytes;
        }
        if let Some(raw_run_bytes) = self.raw_run_bytes {
            options.raw_run_bytes = raw_run_bytes;
        }
        if let Some(chunk_bytes) = self.chunk_bytes {
            options.chunk_bytes = chunk_bytes;
        }

        // apply hard limit overrides
        if let Some(max_bytes) = self.max_bytes {
            options.max_bytes = Some(max_bytes);
        }
        if let Some(max_managed_bytes) = self.max_managed_bytes {
            options.max_managed_bytes = Some(max_managed_bytes);
        }
        if let Some(max_raw_bytes) = self.max_raw_bytes {
            options.max_raw_bytes = Some(max_raw_bytes);
        }
        if let Some(max_shared_bytes) = self.max_shared_bytes {
            options.max_shared_bytes = Some(max_shared_bytes);
        }
    }
}
