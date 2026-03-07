use super::MacosHost;
#[cfg(target_os = "macos")]
use crate::host::HostAdapter;
#[cfg(target_os = "macos")]
use crate::runtime::world::RuntimeId;

#[cfg(target_os = "macos")]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = MacosHost::new(RuntimeId(1));
    let result = host.process_ingress();
    assert!(result.is_ok());
}
