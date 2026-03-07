use std::sync::Arc;

use destack_workspace::{Platform, PlatformHostOptions};

use super::HostEvent;
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// Host platform tag for runtime host routing.
pub type HostPlatform = Platform;

/// Host poll output containing queued events and queue-pressure drops.
#[derive(Debug, Default)]
pub struct HostPollOutcome {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
    /// Host events dropped by queue policy since the previous poll call.
    pub dropped_event_count: u64,
}

/// Shared host contract for runtime integrations.
pub trait HostAdapter: std::fmt::Debug + Send + Sync {
    /// Return the host platform for this host.
    fn platform(&self) -> HostPlatform;

    /// Poll host events with an optional timeout in nanoseconds.
    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome>;

    /// Return one shared wake handle for out-of-band host wakeups.
    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>>;

    /// Configure host integration options on this host.
    fn configure_host_options(&self, host_options: &PlatformHostOptions);

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool;

    /// Service immediately ready native host ingress without blocking.
    fn process_ingress(&self) -> RuntimeResult<bool>;

    /// Return host platform capabilities for this host target.
    fn host_capabilities(&self) -> PlatformCapabilitySet;
}
