use crate::host::HostAdapter;
use crate::host::os::android::AndroidHost;
use crate::runtime::action::HostAction;

/// Report the static Android host actions.
#[test]
fn test_android_host_reports_static_actions() {
    let host = AndroidHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(HostAction::OsLifecycleRead));
    assert!(actions.contains_action(HostAction::OsIntentRead));
    assert!(actions.contains_action(HostAction::OsPower));
    assert!(actions.contains_action(HostAction::OsPermissionRead));
}
