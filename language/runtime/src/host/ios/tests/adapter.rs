use super::IosHost;
#[cfg(target_os = "ios")]
use crate::host::HostAdapter;
#[cfg(target_os = "ios")]
use crate::runtime::world::RuntimeId;

#[cfg(target_os = "ios")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = IosHost::new(RuntimeId(1));
    let result = host.process_ingress();
    assert!(result.is_ok());
}
