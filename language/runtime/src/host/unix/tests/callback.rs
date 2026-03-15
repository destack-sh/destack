use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::unix::ingress::callback::{
    UnixApplicationLifecycle, host_lifecycle_state_for_unix_application,
    unix_notify_permission_result,
};
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};
use crate::runtime::world::RuntimeId;

/// Shared runtime-id allocator for host callback tests.
static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 16_384);

/// Allocate one unique runtime id for this test process.
fn next_test_runtime_id() -> RuntimeId {
    RuntimeId(TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

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
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostQueueRegistry::shared().write().register(
        Platform::Linux,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

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
