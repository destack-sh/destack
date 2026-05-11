use crate::host::HostAdapter;
use crate::host::os::windows::WindowsHost;
use crate::runtime::action::HostAction;

/// Report the static Windows host actions.
#[test]
fn test_windows_host_reports_static_actions() {
    let host = WindowsHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(HostAction::OsLifecycleRead));
    assert!(actions.contains_action(HostAction::OsIntentRead));
    assert!(actions.contains_action(HostAction::OsPower));
    assert!(actions.contains_action(HostAction::OsPermissionRead));
}
