use crate::host::HostEvent;
use crate::runtime::poller::PollerEvent;

use super::RuntimeId;

/// Coordinator-visible arrived work that is not caused by world time advancing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ingress {
    /// Runtime-wide host semantic arrival.
    Host {
        /// Runtime that owns the host integration.
        runtime_id: RuntimeId,
        /// Host event payload.
        event: HostEvent,
    },
    /// Runtime-wide poller arrival.
    Poller {
        /// Runtime that owns the shared poller.
        runtime_id: RuntimeId,
        /// Poller event payload.
        event: PollerEvent,
    },
}
