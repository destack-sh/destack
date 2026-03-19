use std::sync::Arc;

use crate::host::core::registry::next_host_runtime_id;
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::unix::ingress::notify::{
    UnixApplicationLifecycle, host_lifecycle_state_for_unix_application,
    unix_notify_permission_result,
};
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};

#[test]
fn test_map_unix_lifecycle_states() {
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
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostRuntimeRegistry::register_queue(Platform::Linux, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_runtime_id();

    unix_notify_permission_result(runtime_id.0, Platform::Linux, "camera", true).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            permission: "camera".to_string(),
            granted: true,
        })],
    );
}
