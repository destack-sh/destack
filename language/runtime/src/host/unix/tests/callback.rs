use std::sync::Arc;

use super::{
    UnixApplicationLifecycle, host_lifecycle_state_for_unix_application,
    unix_notify_window_available,
};
use crate::host::core::{HostState, host_state_for_runtime, register_host_state};
use crate::host::{HostEvent, HostLifecycleState, HostPlatform, HostWindowEvent};

#[test]
fn test_map_unix_lifecycle_to_host_states() {
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Created),
        HostLifecycleState::Initializing
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Running),
        HostLifecycleState::Running
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Paused),
        HostLifecycleState::Paused
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Stopped),
        HostLifecycleState::Stopped
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Destroyed),
        HostLifecycleState::Destroyed
    );
}

#[test]
fn test_notify_window_available_enqueues_window_event_for_runtime_bridge() {
    let state = Arc::new(HostState::new());
    let registration = register_host_state(HostPlatform::Linux, &state);
    let runtime_id = registration.runtime_id();

    unix_notify_window_available(runtime_id, HostPlatform::Linux, 7).unwrap();

    let poll_result = state.poll_events(Some(0)).unwrap();
    let events = poll_result.events;
    assert_eq!(
        events.as_slice(),
        [HostEvent::Window(HostWindowEvent::WindowAvailable {
            window_id: 7,
        })]
    );
}

#[test]
fn test_notify_window_available_rejects_platform_mismatch_for_runtime_bridge() {
    let state = Arc::new(HostState::new());
    let registration = register_host_state(HostPlatform::Linux, &state);
    let runtime_id = registration.runtime_id();

    let result = unix_notify_window_available(runtime_id, HostPlatform::FreeBsd, 7);
    assert!(result.is_err());

    let resolved_state = host_state_for_runtime(runtime_id, HostPlatform::Linux).unwrap();
    assert!(Arc::ptr_eq(&state, &resolved_state));
}
