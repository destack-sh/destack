#[cfg(target_os = "ios")]
use crate::host::HostBackend;
#[cfg(target_os = "ios")]
use crate::host::ios::IosHost;

#[cfg(target_os = "ios")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = IosHost::new();
    let result = host.process_native_ingress();
    assert!(result.is_ok());
}
