use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::{HostBackend, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// iOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct IosHost;

impl IosHost {
    /// Create one iOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for IosHost {
    fn platform(&self) -> Platform {
        Platform::IOS
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // ios exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Apple run loop
        Ok(apple_message::process_ingress_ready(true))
    }
}
