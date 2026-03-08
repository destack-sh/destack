#[cfg(windows)]
use crate::host::HostBackend;
#[cfg(windows)]
use crate::host::windows::WindowsHost;

#[cfg(windows)]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = WindowsHost::new();
    let result = host.process_native_ingress();
    assert!(result.is_ok());
}
