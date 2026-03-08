#[cfg(target_os = "android")]
use crate::host::HostBackend;
#[cfg(target_os = "android")]
use crate::host::android::AndroidHost;

#[cfg(target_os = "android")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = AndroidHost::new();
    let result = host.process_native_ingress();
    assert!(result.is_ok());
}
