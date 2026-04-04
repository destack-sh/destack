use std::sync::Arc;

use crate::host::os::macos::ingress::notify::{
    MacosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle,
    macos_notify_intent_open_url, macos_notify_location_sample, macos_notify_permission_result,
};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostLocationEvent,
    HostPermissionEvent, HostQueue, HostSessionRegistry, Platform,
};
use crate::platform::os::Permission;
use crate::platform::os::abi_generated::LocationSampleValue;

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
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();

    macos_notify_permission_result(runtime_id.0, "camera", true).unwrap();

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
    let registration =
        HostSessionRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();

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

#[test]
fn test_notify_location_sample_enqueues_location_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
    let sample = test_location_sample();

    macos_notify_location_sample(runtime_id.0, "watch-1", sample).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Location(Box::new(HostLocationEvent {
            watch_id: "watch-1".to_string(),
            sample,
        }))],
    );
}

/// Build one representative location sample payload.
fn test_location_sample() -> LocationSampleValue {
    LocationSampleValue {
        latitude_degrees: 47.3769,
        longitude_degrees: 8.5417,
        altitude_meters: 408.0,
        horizontal_accuracy_meters: 12.0,
        vertical_accuracy_meters: 18.0,
        speed_meters_per_second: 2.5,
        heading_degrees: 180.0,
        timestamp_unix_ns: 123_000_000_000,
    }
}
