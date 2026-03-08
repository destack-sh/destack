use super::super::callback::{
    UnixApplicationLifecycle, host_lifecycle_state_for_unix_application,
    unix_notify_permission_result,
};
use crate::host::core::HostState;
use crate::host::core::registry::register_host_state;
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};
use crate::runtime::world::RuntimeId;
use std::sync::Arc;

#[test]
fn test_map_unix_lifecycle_to_host_states() {
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Created),
        HostLifecycleState::Initializing,
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Running),
        HostLifecycleState::Running,
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Paused),
        HostLifecycleState::Paused,
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Stopped),
        HostLifecycleState::Stopped,
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Destroyed),
        HostLifecycleState::Destroyed,
    );
}

#[test]
fn test_notify_permission_result_enqueues_permission_event_for_runtime_bridge() {
    let state = HostState::new_for_test();
    let registration =
        register_host_state(Platform::Linux, RuntimeId(1), Arc::downgrade(&state), None);
    let runtime_id = registration.runtime_id();

    unix_notify_permission_result(runtime_id.0, Platform::Linux, "camera", true).unwrap();

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
