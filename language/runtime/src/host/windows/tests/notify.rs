use std::sync::Arc;

use crate::host::core::{HostQueue, HostSessionRegistry};
use crate::host::windows::ingress::notify::{
    WindowsApplicationLifecycle, host_lifecycle_state_for_windows_application,
    windows_notify_intent_open_url, windows_notify_location_sample,
    windows_notify_permission_result,
};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostLocationEvent,
    HostPermissionEvent, Platform,
};
use crate::platform::os::Permission;
use crate::platform::os::abi_generated::LocationSampleValue;

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
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::Windows,
        runtime_id,
        Arc::clone(&queue),
        None,
    );
    let runtime_id = registration.host_session_id();

    windows_notify_permission_result(runtime_id.0, "camera", true).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            request_id: None,
            permission: Permission::Camera,
            granted: true,
        })],
    );
}

#[test]
fn test_notify_intent_open_url_enqueues_intent_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::Windows,
        runtime_id,
        Arc::clone(&queue),
        None,
    );
    let runtime_id = registration.host_session_id();

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

#[test]
fn test_notify_location_sample_enqueues_location_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::Windows,
        runtime_id,
        Arc::clone(&queue),
        None,
    );
    let runtime_id = registration.host_session_id();
    let sample = LocationSampleValue {
        latitude_degrees: 1.0,
        longitude_degrees: 2.0,
        altitude_meters: 3.0,
        horizontal_accuracy_meters: 4.0,
        vertical_accuracy_meters: 5.0,
        speed_meters_per_second: 6.0,
        heading_degrees: 7.0,
        timestamp_unix_ns: 8,
    };

    windows_notify_location_sample(runtime_id.0, "watch-1", sample).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Location(Box::new(HostLocationEvent {
            watch_id: "watch-1".to_string(),
            sample,
        }))],
    );
}
