#[cfg(target_os = "macos")]
use super::MacosHost;
#[cfg(target_os = "macos")]
use crate::host::HostAdapter;

#[cfg(target_os = "macos")]
#[test]
fn test_pump_pending_thread_messages_routes_for_host() {
    let host = MacosHost::new();
    let result = host.pump_pending_thread_messages(true);
    assert!(result.is_ok());
}
