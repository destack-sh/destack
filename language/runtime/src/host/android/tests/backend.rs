use crate::host::HostAdapter;
use crate::host::os::android::AndroidHost;
use crate::runtime::capability::PlatformCapability;

/// Report the static Android host capabilities.
#[test]
fn test_android_host_reports_static_capabilities() {
    let host = AndroidHost::new();
    let capabilities = host.static_capabilities();

    assert!(capabilities.contains_capability(PlatformCapability::OsLifecycleRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsIntentRead));
    assert!(capabilities.contains_capability(PlatformCapability::OsPower));
    assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRead));
}
