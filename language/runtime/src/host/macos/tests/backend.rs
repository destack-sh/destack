use crate::host::HostAdapter;
use crate::host::os::macos::MacosHost;
use crate::runtime::action::Action;

/// Report the static macOS host actions.
#[test]
fn test_macos_host_reports_static_actions() {
    let host = MacosHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(Action::OsPowerRead));
}
