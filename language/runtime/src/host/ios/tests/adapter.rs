#[cfg(target_os = "ios")]
use super::IosHost;
#[cfg(target_os = "ios")]
use crate::host::HostAdapter;

#[cfg(target_os = "ios")]
#[test]
fn test_pump_pending_thread_messages_routes_for_host() {
    let host = IosHost::new();
    let result = host.pump_pending_thread_messages(true);
    assert!(result.is_ok());
}
