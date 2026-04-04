use crate::diagnostic::RuntimeResult;
use crate::host::os::windows::ingress::r#loop as windows_message_loop;
use crate::host::os::windows::{ingress, request};
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
    SessionContext,
};
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

    fn advance_native_ingress(&self) -> RuntimeResult<()> {
        // drain one ready slice of the Win32 message queue
        windows_message_loop::drain_ready_ingress();

        Ok(())
    }

    fn advance_session_ingress(&self, context: &SessionContext) -> RuntimeResult<()> {
        ingress::service_windows_ingress(context)
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
