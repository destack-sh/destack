use super::super::callback::{
    MacosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle,
    macos_notify_permission_result,
};
use crate::host::core::HostState;
use crate::host::core::registry::register_host_state;
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};
use crate::runtime::world::RuntimeId;
use std::sync::Arc;

#[test]
fn test_map_application_lifecycle_to_initializing() {
    let state = host_lifecycle_state_for_application_lifecycle(
        MacosApplicationLifecycle::DidFinishLaunching,
    );
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_application_lifecycle_to_running() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::DidBecomeActive);
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_application_lifecycle_to_paused() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::WillResignActive);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_application_lifecycle_to_destroyed() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::WillTerminate);
    assert_eq!(state, HostLifecycleState::Destroyed);
}

#[test]
fn test_notify_permission_result_enqueues_permission_event_for_runtime_bridge() {
    let state = HostState::new_for_test();
    let registration =
        register_host_state(Platform::MacOS, RuntimeId(1), Arc::downgrade(&state), None);
    let runtime_id = registration.runtime_id();

    macos_notify_permission_result(runtime_id.0, "camera", true).unwrap();

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
