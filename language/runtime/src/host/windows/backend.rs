use crate::diagnostic::RuntimeResult;
use crate::host::windows::message as windows_message;
use crate::host::{HostBackend, Platform};
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

impl HostBackend for WindowsHost {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // windows exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Win32 message queue
        Ok(windows_message::process_ingress_ready(true))
    }
}
