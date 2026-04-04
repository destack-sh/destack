use crate::host::HostAdapter;
use crate::host::os::ios::IosHost;
use crate::runtime::capability::PlatformCapability;

/// Report the static iOS host capabilities.
#[test]
fn test_ios_host_reports_static_capabilities() {
    let host = IosHost::new();
    let capabilities = host.static_capabilities();

    assert!(capabilities.contains_capability(PlatformCapability::OsLifecycleRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsIntentRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsPower));
    assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRead));
}
