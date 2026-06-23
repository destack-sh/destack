use std::time::Duration;

use crate::ClientOptions;
use crate::protocol::ProtocolLimits;

/// Options for connecting to a workspace service.
#[derive(Clone)]
pub struct ConnectOptions {
    /// Client handshake options.
    pub client: ClientOptions,
    /// Protocol limits used while connecting.
    pub limits: ProtocolLimits,
    /// Delay between connection attempts.
    pub retry_delay: Duration,
    /// Maximum time to wait for a workspace service.
    pub timeout: Duration,
}

impl std::fmt::Debug for ConnectOptions {
    /// Format the visible workspace service connect options.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConnectOptions")
            .field("client", &self.client)
            .field("limits", &self.limits)
            .field("retry_delay", &self.retry_delay)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for ConnectOptions {
    /// Return default connect options.
    fn default() -> Self {
        Self {
            client: ClientOptions::default(),
            limits: ProtocolLimits::default(),
            retry_delay: Duration::from_millis(50),
            timeout: Duration::from_secs(3),
        }
    }
}
