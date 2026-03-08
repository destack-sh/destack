use serde::{Deserialize, Serialize};

/// Telemetry settings for a VM isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryOptions {
    /// Collect execution statistics during runtime.
    pub collect_stats: bool,
}

impl Default for TelemetryOptions {
    fn default() -> Self {
        // collect stats by default
        Self {
            collect_stats: true,
        }
    }
}
