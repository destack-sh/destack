use destack_heap::{DEFAULT_GC_GROWTH_PERCENT, DEFAULT_YOUNG_SIZE_BYTES};
use serde::{Deserialize, Serialize};

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct HeapOptions {
    /// Soft memory limit inherited by local and shared heap collectors.
    pub memory_limit_bytes: Option<u64>,
    /// The proportional heap growth target percentage.
    pub growth_percent: u32,
    /// Worker-local heap policy.
    pub local: LocalHeapOptions,
    /// Runtime-shared heap policy.
    pub shared: SharedHeapOptions,
}

impl Default for HeapOptions {
    fn default() -> Self {
        Self {
            memory_limit_bytes: None,
            growth_percent: DEFAULT_GC_GROWTH_PERCENT,
            local: LocalHeapOptions::default(),
            shared: SharedHeapOptions::default(),
        }
    }
}

/// Runtime local-heap policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LocalHeapOptions {
    /// The byte width for worker-local young space.
    pub young_size_bytes: usize,
    /// Minimum heap bytes before normal growth pacing applies.
    pub min_bytes: Option<u64>,
    /// Hard limit for total retained local heap bytes.
    pub max_bytes: Option<u64>,
}

impl Default for LocalHeapOptions {
    fn default() -> Self {
        Self {
            young_size_bytes: DEFAULT_YOUNG_SIZE_BYTES,
            min_bytes: None,
            max_bytes: None,
        }
    }
}

/// Runtime shared-heap policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SharedHeapOptions {
    /// Minimum heap bytes before normal growth pacing applies.
    pub min_bytes: Option<u64>,
    /// Hard limit for total retained shared heap bytes.
    pub max_bytes: Option<u64>,
}
