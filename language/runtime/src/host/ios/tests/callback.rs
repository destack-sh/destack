use std::sync::Arc;

use super::{
    IosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle,
    ios_notify_window_available,
};
use crate::host::core::{HostState, register_host_state};
use crate::host::{HostEvent, HostLifecycleState, HostPlatform, HostWindowEvent};

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

#[test]
fn test_notify_window_available_enqueues_window_event_for_runtime_bridge() {
    let state = Arc::new(HostState::new());
    let registration = register_host_state(HostPlatform::IOS, &state, None);
    let runtime_id = registration.runtime_id();

    ios_notify_window_available(runtime_id, 21).unwrap();

    let poll_result = state.poll_events(Some(0)).unwrap();
    let events = poll_result.events;
    assert_eq!(
        events.as_slice(),
        [HostEvent::Window(HostWindowEvent::WindowAvailable {
            window_id: 21,
        })]
    );
}

#[test]
fn test_notify_window_available_rejects_unknown_runtime_id() {
    let result = ios_notify_window_available(0, 21);
    assert!(result.is_err());
}
