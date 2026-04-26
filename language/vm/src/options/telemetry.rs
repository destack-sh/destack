use serde::{Deserialize, Serialize};

/// Telemetry settings for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryOptions {}
