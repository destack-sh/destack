use super::HostPlatform;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return host capabilities currently implemented by default hosts.
pub(crate) fn default_host_capabilities(_platform: HostPlatform) -> PlatformCapabilitySet {
    let mut host_capabilities = PlatformCapabilitySet::new();

    // lifecycle and power signals are routed through the host event bridge
    host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
    host_capabilities.insert_capability(PlatformCapability::OsPower);

    // window events are routed through host window callbacks
    host_capabilities.insert_capability(PlatformCapability::DisplayWindowEvents);

    // permission result tracking is routed through host permission callbacks
    host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

    host_capabilities
}
