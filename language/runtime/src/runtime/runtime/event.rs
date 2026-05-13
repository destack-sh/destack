use crate::host::HostEvent;
use crate::host::poller::PollerEvent;

/// Runtime-local event delivered outside world time advancement.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RuntimeEvent {
    /// Host event.
    Host {
        /// Host event payload.
        event: HostEvent,
    },
    /// Poller event.
    Poller {
        /// Poller event payload.
        event: PollerEvent,
    },
}
