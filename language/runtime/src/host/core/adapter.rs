#[cfg(any(test, feature = "execution"))]
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::core::registry::HostSessionId;
use crate::host::core::request::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostSessionContext,
};
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

/// Shared process-global host adapter contract for one host integration family.
///
/// This is the runtime-facing host boundary above request transport and platform glue.
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

    /// Return dynamic session capabilities for one attached runtime session.
    fn session_capabilities(&self, _host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Service immediately ready native ingress without blocking.
    fn process_native_ingress(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Service runtime-owned host ingress for one attached session.
    fn process_runtime_ingress(&self, _context: &HostSessionContext) -> RuntimeResult<()> {
        Ok(())
    }

    /// Submit one runtime-owned host request through this adapter.
    fn submit_request(
        &self,
        _context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        Err(not_supported(request.operation_name()))
    }
}

/// Adapter wrapper that suppresses ambient native ingress.
#[cfg(any(test, feature = "execution"))]
#[derive(Debug)]
struct NativeIngressDisabledHostAdapter {
    /// Wrapped host adapter.
    adapter: Arc<dyn HostAdapter>,
}

#[cfg(any(test, feature = "execution"))]
impl HostAdapter for NativeIngressDisabledHostAdapter {
    fn platform(&self) -> Platform {
        self.adapter.platform()
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        self.adapter.static_capabilities()
    }

    fn session_capabilities(&self, host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        self.adapter.session_capabilities(host_runtime_id)
    }

    fn is_process_main_context(&self) -> bool {
        self.adapter.is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Service runtime-owned host ingress through the wrapped adapter.
    fn process_runtime_ingress(&self, context: &HostSessionContext) -> RuntimeResult<()> {
        self.adapter.process_runtime_ingress(context)
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        self.adapter.submit_request(context, request)
    }
}

/// Wrap one host adapter so ambient native ingress is suppressed.
#[cfg(any(test, feature = "execution"))]
pub(crate) fn without_native_ingress(adapter: Arc<dyn HostAdapter>) -> Arc<dyn HostAdapter> {
    Arc::new(NativeIngressDisabledHostAdapter { adapter })
}
