use crate::host::HostAdapter;
use crate::host::os::macos::MacosHost;
use crate::runtime::capability::PlatformCapability;

/// Report the static macOS host capabilities.
#[test]
fn test_macos_host_reports_static_capabilities() {
    let host = MacosHost::new();
    let capabilities = host.static_capabilities();

    assert!(capabilities.contains_capability(PlatformCapability::OsLifecycleRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsIntentRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsPower));
    assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRead));
}
