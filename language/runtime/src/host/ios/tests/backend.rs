use crate::host::HostAdapter;
use crate::host::os::ios::IosHost;
use crate::runtime::action::HostAction;

/// Report the static iOS host actions.
#[test]
fn test_ios_host_reports_static_actions() {
    let host = IosHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(HostAction::OsLifecycleRead));
    assert!(actions.contains_action(HostAction::OsIntentRead));
    assert!(actions.contains_action(HostAction::OsPower));
    assert!(actions.contains_action(HostAction::OsPermissionRead));
}
