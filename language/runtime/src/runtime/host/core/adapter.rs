use std::sync::Arc;

use destack_workspace::Platform;

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

    /// Return the active host capability set.
    fn capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    /// Return service surfaces exposed by this host adapter.
    fn services(&self) -> &HostServices;
}
