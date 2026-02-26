use std::sync::Arc;

use destack_workspace::{Platform, PlatformHostOptions};

use super::{HostEvent, HostServices};
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// Host platform tag for runtime adapter routing.
pub type HostPlatform = Platform;

/// Shared host adapter contract for runtime integrations.
pub trait HostAdapter: std::fmt::Debug + Send + Sync {
    /// Return the host platform for this adapter.
    fn platform(&self) -> HostPlatform;

    /// Poll host adapter events with an optional timeout in nanoseconds.
    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>>;

    /// Return one shared wake handle for out-of-band host wakeups.
    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>>;

    /// Configure host integration options on this adapter.
    fn configure_host_options(&self, _host_options: &PlatformHostOptions) {}

    /// Return the callback runtime id used by native host callback routing.
    fn callback_runtime_id(&self) -> Option<u64> {
        None
    }

    /// Take the number of dropped host events observed by this adapter.
    fn take_dropped_event_count(&self) -> u64 {
        0
    }

    /// Return host platform capabilities for this adapter target.
    fn host_capabilities(&self) -> PlatformCapabilitySet;

    /// Return service surfaces exposed by this host adapter.
    fn services(&self) -> &HostServices;
}
