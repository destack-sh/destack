use crate::host::HostAdapter;
use crate::host::os::windows::WindowsHost;
use crate::runtime::action::Action;

/// Report the static Windows host actions.
#[test]
fn test_windows_host_reports_static_actions() {
    let host = WindowsHost::new();
    let actions = host.static_actions();

    assert!(actions.contains_action(Action::OsPowerRead));
}
