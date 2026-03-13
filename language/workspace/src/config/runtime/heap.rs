use serde::{Deserialize, Serialize};

/// Runtime heap configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapOptions {
    /// Growth target percentage.
    pub growth_percent: u32,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// Dedicated managed span threshold in values.
    pub managed_large_span_values: usize,
    /// Dedicated raw span threshold in bytes.
    pub raw_large_span_bytes: usize,
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
            growth_percent: 100,
            soft_limit_bytes: None,
            initial_bytes: None,
            managed_large_span_values: 256,
            raw_large_span_bytes: 4096,
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
    /// Growth target percentage.
    pub growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub initial_bytes: Option<u64>,
    /// Dedicated managed span threshold in values.
    pub managed_large_span_values: Option<usize>,
    /// Dedicated raw span threshold in bytes.
    pub raw_large_span_bytes: Option<usize>,
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
        if let Some(managed_large_span_values) = self.managed_large_span_values {
            options.managed_large_span_values = managed_large_span_values;
        }
        if let Some(raw_large_span_bytes) = self.raw_large_span_bytes {
            options.raw_large_span_bytes = raw_large_span_bytes;
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
