use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::macos::{macos_request_capabilities, submit_macos_request};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};
use crate::runtime::world::RuntimeId;

/// macOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct MacosHost;

impl MacosHost {
    /// Create one macOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for MacosHost {
    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // macos exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn session_capabilities(&self, _runtime_id: RuntimeId) -> PlatformCapabilitySet {
        macos_request_capabilities()
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Apple run loop
        Ok(apple_message::process_ingress_ready(true))
    }

    fn submit_request(
        &self,
        _context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        submit_macos_request(request)
    }
}
