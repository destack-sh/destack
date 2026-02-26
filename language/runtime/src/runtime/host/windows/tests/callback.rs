use super::{WindowsApplicationLifecycle, host_lifecycle_state_for_windows_application};
use crate::runtime::host::HostLifecycleState;

#[test]
fn test_map_windows_lifecycle_to_initializing() {
    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Created);
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_windows_lifecycle_to_running() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Activated);
    assert_eq!(state, HostLifecycleState::Running);

    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Resumed);
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_windows_lifecycle_to_paused() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Suspended);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_windows_lifecycle_to_stopped() {
    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Stopping);
    assert_eq!(state, HostLifecycleState::Stopped);
}

#[test]
fn test_map_windows_lifecycle_to_destroyed() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Destroyed);
    assert_eq!(state, HostLifecycleState::Destroyed);
}
