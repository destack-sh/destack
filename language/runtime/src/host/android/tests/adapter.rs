use super::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::HostAdapter;
#[cfg(target_os = "android")]
use crate::runtime::world::RuntimeId;

#[cfg(target_os = "android")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = AndroidHost::new(RuntimeId(1));
    let result = host.process_ingress();
    assert!(result.is_ok());
}
