use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::macos::ingress::callback::host_lifecycle_state_for_application_lifecycle;
use crate::host::macos::{
    MacosApplicationLifecycle, macos_notify_intent_open_url, macos_notify_permission_result,
};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostPermissionEvent,
    Platform,
};
use crate::runtime::world::RuntimeId;

/// Shared runtime-id allocator for host callback tests.
static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 8192);

/// Allocate one unique runtime id for this test process.
fn next_test_runtime_id() -> RuntimeId {
    RuntimeId(TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

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
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostQueueRegistry::shared().write().register(
        Platform::MacOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

    macos_notify_permission_result(runtime_id.0, "camera", true).unwrap();

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
        Platform::MacOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

    macos_notify_intent_open_url(
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
