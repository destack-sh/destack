use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Host poll output containing queued events and queue-pressure drops.
#[derive(Debug, Default)]
pub struct HostPollOutcome {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
    /// Host events dropped by queue policy since the previous poll call.
    pub dropped_event_count: u64,
}

/// Shared host contract for runtime integrations.
pub(crate) trait HostBackend: std::fmt::Debug + Send + Sync {
    /// Return the host platform for this host.
    fn platform(&self) -> Platform;

    /// Return host platform capabilities for this host target.
    fn host_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // power state is a broadly available host binding family
        host_capabilities.insert_capability(PlatformCapability::OsPower);

        host_capabilities
    }

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Service immediately ready native host ingress without blocking.
    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        Ok(false)
    }
}
