use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::windows::ingress::callback::host_lifecycle_state_for_windows_application;
use crate::host::windows::{
    WindowsApplicationLifecycle, windows_notify_intent_open_url, windows_notify_permission_result,
};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostPermissionEvent,
    Platform,
};
use crate::runtime::world::RuntimeId;

/// Shared runtime-id allocator for host callback tests.
static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 4096);

/// Allocate one unique runtime id for this test process.
fn next_test_runtime_id() -> RuntimeId {
    RuntimeId(TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

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
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostQueueRegistry::shared().write().register(
        Platform::Windows,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

    windows_notify_permission_result(runtime_id.0, "camera", true).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            permission: "camera".to_string(),
            granted: true,
        })],
    );
}

#[test]
fn test_notify_intent_open_url_enqueues_intent_event_for_runtime_bridge() {
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostQueueRegistry::shared().write().register(
        Platform::Windows,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

    windows_notify_intent_open_url(
        runtime_id.0,
        Some("com.example.source"),
        "https://example.com",
    )
    .unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Intent(HostIntentEvent {
            source: Some("com.example.source".to_string()),
            payload: HostIntentPayload::OpenUrl {
                url: "https://example.com".to_string(),
            },
        })],
    );
}
