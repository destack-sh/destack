use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::core::registry::HostRuntimeId;
use crate::host::core::request::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::{HostEvent, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Host poll output containing queued events and queue-pressure drops.
#[derive(Debug, Default)]
pub struct HostPollOutcome {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
    /// Host events dropped by queue policy since the previous poll call.
    pub dropped_event_count: u64,
}

/// Shared process-global host adapter contract for runtime integrations.
pub(crate) trait HostAdapter: std::fmt::Debug + Send + Sync {
    /// Return the host platform for this adapter.
    fn platform(&self) -> Platform;

    /// Return static host capabilities for this adapter target.
    fn static_capabilities(&self) -> PlatformCapabilitySet {
        let mut capabilities = PlatformCapabilitySet::new();

        // power state is a broadly available host binding family
        capabilities.insert_capability(PlatformCapability::OsPower);

        capabilities
    }

    /// Return dynamic session capabilities for one specific runtime.
    fn session_capabilities(&self, _host_runtime_id: HostRuntimeId) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Service immediately ready native host ingress without blocking.
    fn process_native_ingress(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Submit one host request through this adapter.
    fn submit_request(
        &self,
        _context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        Err(not_supported(request.operation_name()))
    }
}
