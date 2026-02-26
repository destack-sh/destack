use super::super::callback::{AndroidActivityLifecycle, host_lifecycle_state_for_android_activity};
use crate::runtime::host::HostLifecycleState;

#[test]
fn test_map_activity_lifecycle_to_initializing() {
    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Created);
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_activity_lifecycle_to_running() {
    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Started);
    assert_eq!(state, HostLifecycleState::Running);

    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Resumed);
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_activity_lifecycle_to_paused() {
    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Paused);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_activity_lifecycle_to_stopped() {
    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Stopped);
    assert_eq!(state, HostLifecycleState::Stopped);
}

#[test]
fn test_map_activity_lifecycle_to_destroyed() {
    let state = host_lifecycle_state_for_android_activity(AndroidActivityLifecycle::Destroyed);
    assert_eq!(state, HostLifecycleState::Destroyed);
}
