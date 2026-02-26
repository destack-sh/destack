use std::time::Duration;

use destack_workspace::{DEFAULT_DAEMON_IDLE_SHUTDOWN_MS, DsConfigDaemonOptions};

/// Shutdown policy options for the daemon server.
#[derive(Debug, Clone)]
pub struct DaemonShutdownOptions {
    /// Idle shutdown timeout.
    pub idle_shutdown: Option<Duration>,
    /// Poll interval for idle checks.
    pub idle_poll: Duration,
}

impl DaemonShutdownOptions {
    /// Build shutdown options from daemon config.
    pub fn from_config(config: &DsConfigDaemonOptions) -> Self {
        // map idle shutdown config to a duration
        let idle_shutdown = config.idle_shutdown_ms.map(Duration::from_millis);

        // return shutdown options
        Self {
            idle_shutdown,
            idle_poll: Duration::from_millis(250),
        }
    }
}

impl Default for DaemonShutdownOptions {
    /// Return default daemon shutdown options.
    fn default() -> Self {
        // return shutdown options
        Self {
            idle_shutdown: Some(Duration::from_millis(DEFAULT_DAEMON_IDLE_SHUTDOWN_MS)),
            idle_poll: Duration::from_millis(250),
        }
    }
}
