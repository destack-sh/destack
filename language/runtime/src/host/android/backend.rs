use crate::diagnostic::RuntimeResult;
use crate::host::android::message as android_message;
use crate::host::{HostBackend, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Android host implementation.
#[derive(Debug, Default)]
pub(crate) struct AndroidHost;

impl AndroidHost {
    /// Create one Android host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for AndroidHost {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // android exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Android looper
        Ok(android_message::process_ingress_ready(true))
    }
}
