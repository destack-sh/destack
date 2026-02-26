use super::{IosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle};
use crate::runtime::host::HostLifecycleState;

#[test]
fn test_map_application_lifecycle_to_initializing() {
    let state =
        host_lifecycle_state_for_application_lifecycle(IosApplicationLifecycle::DidFinishLaunching);
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_application_lifecycle_to_running() {
    let state =
        host_lifecycle_state_for_application_lifecycle(IosApplicationLifecycle::DidBecomeActive);
    assert_eq!(state, HostLifecycleState::Running);

    let state = host_lifecycle_state_for_application_lifecycle(
        IosApplicationLifecycle::WillEnterForeground,
    );
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_application_lifecycle_to_paused() {
    let state =
        host_lifecycle_state_for_application_lifecycle(IosApplicationLifecycle::WillResignActive);
    assert_eq!(state, HostLifecycleState::Paused);

    let state =
        host_lifecycle_state_for_application_lifecycle(IosApplicationLifecycle::DidEnterBackground);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_application_lifecycle_to_destroyed() {
    let state =
        host_lifecycle_state_for_application_lifecycle(IosApplicationLifecycle::WillTerminate);
    assert_eq!(state, HostLifecycleState::Destroyed);
}
