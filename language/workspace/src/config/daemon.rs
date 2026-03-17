use serde::Deserialize;

/// Daemon configuration options.
#[derive(Debug, Clone)]
pub struct DaemonOptions {
    /// Idle shutdown timeout in milliseconds, or None to disable.
    pub idle_shutdown_ms: Option<u64>,
}

/// Default idle shutdown timeout for the daemon.
pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_MS: u64 = 600_000;

impl Default for DaemonOptions {
    /// Return default daemon options.
    fn default() -> Self {
        Self {
            idle_shutdown_ms: Some(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS),
        }
    }
}

/// Daemon options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DaemonJson {
    /// Idle shutdown timeout in milliseconds.
    pub idle_shutdown_ms: Option<u64>,
}

impl From<&DaemonJson> for DaemonOptions {
    /// Convert daemon JSON options into normalized options.
    fn from(json: &DaemonJson) -> Self {
        // normalize the idle shutdown setting
        let idle_shutdown_ms = match json.idle_shutdown_ms {
            Some(0) => None,
            Some(value) => Some(value),
            None => Some(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS),
        };

        Self { idle_shutdown_ms }
    }
}
