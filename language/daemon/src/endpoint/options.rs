use std::time::Duration;

use tspp_rpc::ConnectionOptions;

/// Options for connecting to one daemon endpoint.
#[derive(Debug, Clone)]
pub struct DaemonConnectOptions {
    /// RPC negotiation options.
    pub rpc: ConnectionOptions,
    /// Delay between launch connection attempts.
    pub retry_delay: Duration,
    /// Maximum duration to await a launched daemon.
    pub timeout: Duration,
}

impl Default for DaemonConnectOptions {
    /// Create default daemon connection options.
    fn default() -> Self {
        Self {
            rpc: ConnectionOptions::new("tspp-daemon-client"),
            retry_delay: Duration::from_millis(50),
            timeout: Duration::from_secs(3),
        }
    }
}
