use super::WindowsHost;
#[cfg(windows)]
use crate::host::HostAdapter;
#[cfg(windows)]
use crate::runtime::world::RuntimeId;

#[cfg(windows)]
#[test]
fn test_process_ingress_routes_for_host() {
    let host = WindowsHost::new(RuntimeId(1));
    let result = host.process_ingress();
    assert!(result.is_ok());
}
