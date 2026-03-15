use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::windows::ingress::message as windows_message;
use crate::host::windows::{submit_windows_request, windows_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};
use crate::runtime::world::RuntimeId;

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

    fn session_capabilities(&self, _runtime_id: RuntimeId) -> PlatformCapabilitySet {
        windows_request_capabilities()
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Win32 message queue
        Ok(windows_message::process_ingress_ready(true))
    }

    fn submit_request(
        &self,
        _context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        submit_windows_request(request)
    }
}
