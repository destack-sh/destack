#[cfg(target_os = "android")]
use super::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::HostAdapter;

#[cfg(target_os = "android")]
#[test]
fn test_pump_pending_thread_messages_routes_for_host() {
    let host = AndroidHost::new();
    let result = host.pump_pending_thread_messages(true);
    assert!(result.is_ok());
}
