use serde::{Deserialize, Serialize};

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapOptions {
    /// Heap growth target percentage.
    pub heap_growth_percent: u32,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
}

impl Default for HeapOptions {
    fn default() -> Self {
        Self {
            heap_growth_percent: 100,
            heap_soft_limit_bytes: None,
            heap_initial_bytes: None,
            max_bytes: None,
            max_managed_bytes: None,
            max_raw_bytes: None,
        }
    }
}

/// Runtime heap configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeapOptionsJson {
    /// Heap growth target percentage.
    pub heap_growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// Hard limit for total retained heap bytes.
    pub max_bytes: Option<u64>,
    /// Hard limit for retained managed heap bytes.
    pub max_managed_bytes: Option<u64>,
    /// Hard limit for retained raw heap bytes.
    pub max_raw_bytes: Option<u64>,
}

impl HeapOptionsJson {
    /// Apply heap overrides to a base set of options.
    pub fn apply_to(&self, options: &mut HeapOptions) {
        // apply pacing overrides
        if let Some(heap_growth_percent) = self.heap_growth_percent {
            options.heap_growth_percent = heap_growth_percent;
        }

        // apply memory limit overrides
        if let Some(heap_soft_limit_bytes) = self.heap_soft_limit_bytes {
            options.heap_soft_limit_bytes = Some(heap_soft_limit_bytes);
        }
        if let Some(heap_initial_bytes) = self.heap_initial_bytes {
            options.heap_initial_bytes = Some(heap_initial_bytes);
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
    }
}
