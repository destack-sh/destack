use std::fmt;
use std::time::Duration;

use tspp_rpc::ConnectionOptions;

use super::constants::DEFAULT_IDLE_TIMEOUT;

/// Options for one daemon process.
#[derive(Clone)]
pub struct DaemonOptions {
    /// RPC negotiation and resource options.
    pub rpc: ConnectionOptions,
    /// Time before an idle daemon shuts down.
    pub idle_timeout: Option<Duration>,
}

impl fmt::Debug for DaemonOptions {
    /// Format visible daemon options.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DaemonOptions")
            .field("rpc", &self.rpc)
            .field("idle_timeout", &self.idle_timeout)
            .finish()
    }
}

impl Default for DaemonOptions {
    /// Create default daemon options.
    fn default() -> Self {
        Self {
            rpc: ConnectionOptions::new("tspp-daemon"),
            idle_timeout: Some(DEFAULT_IDLE_TIMEOUT),
        }
    }
}
