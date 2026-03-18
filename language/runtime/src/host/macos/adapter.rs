use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
use crate::host::macos::{macos_request_capabilities, submit_macos_request};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

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

    fn session_capabilities(&self, _host_runtime_id: HostRuntimeId) -> PlatformCapabilitySet {
        macos_request_capabilities()
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        // service one ready slice of the Apple run loop
        apple_message::process_ingress_ready(true);

        Ok(())
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        submit_macos_request(context, request)
    }
}
