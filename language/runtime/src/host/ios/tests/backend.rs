use crate::host::HostAdapter;
use crate::host::os::ios::IosHost;
use crate::runtime::action::Action;

/// Report the static iOS host actions.
#[test]
fn test_ios_host_reports_static_actions() {
    let host = IosHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(Action::OsPowerRead));
}
