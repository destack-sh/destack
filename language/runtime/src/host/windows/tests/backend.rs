use crate::host::HostAdapter;
use crate::host::windows::WindowsHost;
use crate::runtime::capability::PlatformCapability;

/// Report the static Windows host capabilities.
#[test]
fn test_windows_host_reports_static_capabilities() {
    let host = WindowsHost::new();
    let capabilities = host.static_capabilities();

    assert!(capabilities.contains_capability(PlatformCapability::OsLifecycleRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsIntentRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsPower));
    assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRead));
}
