use crate::diagnostic::RuntimeResult;
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostSessionContext, HostSessionId,
};
use crate::host::windows::ingress::message as windows_message;
use crate::host::windows::{ingress, request};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Windows host implementation.
#[derive(Debug, Default)]
pub(crate) struct WindowsHost;

impl WindowsHost {
    /// Create one Windows host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for WindowsHost {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // windows exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn session_capabilities(&self, _host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        request::request_capabilities()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        // service one ready slice of the Win32 message queue
        windows_message::process_ingress_ready(true);

        Ok(())
    }

    fn process_runtime_ingress(&self, context: &HostSessionContext) -> RuntimeResult<()> {
        ingress::service_windows_ingress(context)
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
