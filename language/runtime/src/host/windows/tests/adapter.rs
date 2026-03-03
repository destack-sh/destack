#[cfg(windows)]
use super::WindowsHost;
#[cfg(windows)]
use crate::host::HostAdapter;

#[cfg(windows)]
#[test]
fn test_pump_pending_thread_messages_routes_for_host() {
    let host = WindowsHost::new();
    let result = host.pump_pending_thread_messages(true);
    assert!(result.is_ok());
}
