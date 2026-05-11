use crate::host::HostAdapter;
use crate::host::os::macos::MacosHost;
use crate::runtime::action::HostAction;

/// Report the static macOS host actions.
#[test]
fn test_macos_host_reports_static_actions() {
    let host = MacosHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(HostAction::OsLifecycleRead));
    assert!(actions.contains_action(HostAction::OsIntentRead));
    assert!(actions.contains_action(HostAction::OsPower));
    assert!(actions.contains_action(HostAction::OsPermissionRead));
}
