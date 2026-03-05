use serde::{Deserialize, Serialize};

/// Garbage collector logging verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GcLogging {
    /// Disable GC logging.
    #[default]
    Off,
    /// Emit summary GC events.
    Summary,
    /// Emit verbose GC events.
    Verbose,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GcOptions {
    /// Whether the garbage collector is enabled.
    pub enabled: bool,
    /// Heap growth target percentage.
    pub heap_growth_percent: u32,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// GC logging verbosity.
    pub logging: GcLogging,
}

impl Default for GcOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            heap_growth_percent: 100,
            heap_soft_limit_bytes: None,
            heap_initial_bytes: None,
            logging: GcLogging::Off,
        }
    }
}
/// Runtime GC options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct GcOptionsJson {
    /// Whether the garbage collector is enabled.
    pub enabled: Option<bool>,
    /// Heap growth target percentage.
    pub heap_growth_percent: Option<u32>,
    /// Soft heap limit in bytes.
    pub heap_soft_limit_bytes: Option<u64>,
    /// Initial heap size hint in bytes.
    pub heap_initial_bytes: Option<u64>,
    /// GC logging verbosity.
    pub logging: Option<GcLoggingJson>,
}

impl GcOptionsJson {
    /// Apply GC overrides to a base set of options.
    pub fn apply_to(&self, options: &mut GcOptions) {
        // apply enablement overrides
        if let Some(enabled) = self.enabled {
            options.enabled = enabled;
        }

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

        // apply logging overrides
        if let Some(logging) = self.logging {
            options.logging = GcLogging::from(logging);
        }
    }
}
/// GC logging for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum GcLoggingJson {
    /// Disable GC logging.
    Off,
    /// Emit summary GC events.
    Summary,
    /// Emit verbose GC events.
    Verbose,
}

impl From<GcLoggingJson> for GcLogging {
    fn from(value: GcLoggingJson) -> Self {
        match value {
            GcLoggingJson::Off => GcLogging::Off,
            GcLoggingJson::Summary => GcLogging::Summary,
            GcLoggingJson::Verbose => GcLogging::Verbose,
        }
    }
}
