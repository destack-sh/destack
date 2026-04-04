use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return the static macOS host capabilities.
pub(crate) fn static_capabilities() -> PlatformCapabilitySet {
    let mut host_capabilities = PlatformCapabilitySet::new();

    host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
    host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
    host_capabilities.insert_capability(PlatformCapability::OsPower);
    host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

    host_capabilities
}
