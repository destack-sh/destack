use super::super::callback::{
    WindowsApplicationLifecycle, host_lifecycle_state_for_windows_application,
    windows_notify_permission_result,
};
use crate::host::core::HostState;
use crate::host::core::registry::register_host_state;
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};
use crate::runtime::world::RuntimeId;
use std::sync::Arc;

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

#[test]
fn test_notify_permission_result_enqueues_permission_event_for_runtime_bridge() {
    let state = HostState::new_for_test();
    let registration = register_host_state(
        Platform::Windows,
        RuntimeId(1),
        Arc::downgrade(&state),
        None,
    );
    let runtime_id = registration.runtime_id();

    windows_notify_permission_result(runtime_id.0, "camera", true).unwrap();

    let poll_result = state.poll_events(Some(0)).unwrap();
    let events = poll_result.events;
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            permission: "camera".to_string(),
            granted: true,
        })],
    );
}
