use std::sync::Arc;

use crate::host::core::registry::next_host_runtime_id;
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::macos::ingress::notify::host_lifecycle_state_for_application_lifecycle;
use crate::host::macos::{
    MacosApplicationLifecycle, macos_notify_intent_open_url, macos_notify_location_sample,
    macos_notify_permission_result,
};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostLocationEvent,
    HostPermissionEvent, Platform,
};
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
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostRuntimeRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_runtime_id();

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
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostRuntimeRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_runtime_id();

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
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostRuntimeRegistry::register_queue(Platform::MacOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_runtime_id();
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
        altitude_meters: Some(408.0),
        horizontal_accuracy_meters: Some(12.0),
        vertical_accuracy_meters: Some(18.0),
        speed_meters_per_second: Some(2.5),
        heading_degrees: Some(180.0),
        timestamp_unix_ns: 123_000_000_000,
    }
}
