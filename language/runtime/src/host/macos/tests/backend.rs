#[cfg(target_os = "macos")]
use crate::host::HostBackend;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;

#[cfg(target_os = "macos")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = MacosHost::new();
    let result = host.process_native_ingress();
    assert!(result.is_ok());
}
