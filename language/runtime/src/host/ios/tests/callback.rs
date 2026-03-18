use std::sync::Arc;

use crate::host::core::registry::next_host_runtime_id;
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::host::ios::ingress::callback::host_lifecycle_state_for_application_lifecycle;
use crate::host::ios::{
    IosApplicationLifecycle, ios_notify_background_event, ios_notify_intent_open_url,
    ios_notify_location_sample, ios_notify_media_event, ios_notify_notification_event,
    ios_notify_permission_result,
};
use crate::host::{
    HostBackgroundEvent, HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState,
    HostLocationEvent, HostMediaEvent, HostMediaEventKind, HostNotificationEvent,
    HostPermissionEvent, Platform,
};
use crate::platform::os::{
    BackgroundEventMetadataValue, BackgroundEventValue, BackgroundTaskReadyEventValue,
    LocationSampleValue, MediaAddedEventValue, MediaAssetSummaryValue, MediaEventMetadata,
    MediaEventValue, NotificationDeliveredEventValue, NotificationEventMetadataValue,
    NotificationEventValue, NotificationImmediateTriggerValue, NotificationPriority,
    NotificationRequestValue, NotificationTriggerValue,
};

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
fn test_notify_permission_result_enqueues_permission_event_for_runtime_bridge() {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();

    ios_notify_permission_result(runtime_id.0, "camera", true).unwrap();

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
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();

    ios_notify_intent_open_url(
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
fn test_notify_notification_event_enqueues_notification_event_for_runtime_bridge() {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();
    let event = test_notification_event();

    ios_notify_notification_event(runtime_id.0, event.clone()).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Notification(Box::new(HostNotificationEvent {
            event,
        }))],
    );
}

#[test]
fn test_notify_background_event_enqueues_background_event_for_runtime_bridge() {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();
    let event = test_background_event();

    ios_notify_background_event(runtime_id.0, event.clone()).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Background(Box::new(HostBackgroundEvent {
            event
        }))],
    );
}

#[test]
fn test_notify_location_sample_enqueues_location_event_for_runtime_bridge() {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();
    let sample = test_location_sample();

    ios_notify_location_sample(runtime_id.0, "watch-1", sample).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Location(Box::new(HostLocationEvent {
            watch_id: "watch-1".to_string(),
            sample,
        }))],
    );
}

#[test]
fn test_notify_media_event_enqueues_media_event_for_runtime_bridge() {
    let runtime_id = next_host_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::IOS,
        runtime_id,
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.host_runtime_id();
    let asset = MediaAssetSummaryValue {
        id: "asset-1".to_string(),
        uri: "ph://asset-1".to_string(),
        filename: "photo.jpg".to_string(),
        kind: crate::platform::os::MediaAssetKind::Image,
        content_type: Some("image/jpeg".to_string()),
        size_bytes: Some(1024),
        created_unix_ns: Some(42),
        modified_unix_ns: Some(43),
    };
    let event = MediaEventValue::MediaAddedEvent(MediaAddedEventValue {
        kind: "added".to_string(),
        metadata: MediaEventMetadata {
            timestamp_ns: 42,
            sequence: 7,
        },
        asset: asset.clone(),
    });

    ios_notify_media_event(runtime_id.0, "watch-1", event).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Media(Box::new(HostMediaEvent {
            watch_id: "watch-1".to_string(),
            kind: HostMediaEventKind::Added,
            asset,
        }))],
    );
}

/// Build one representative notification event payload.
fn test_notification_event() -> NotificationEventValue {
    NotificationEventValue::NotificationDeliveredEvent(NotificationDeliveredEventValue {
        kind: "delivered".to_string(),
        metadata: NotificationEventMetadataValue {
            timestamp_ns: 42,
            sequence: 7,
            id: "notification-1".to_string(),
            request: NotificationRequestValue {
                title: "Title".to_string(),
                subtitle: None,
                body: "Body".to_string(),
                tag: "tag".to_string(),
                channel_id: None,
                priority: NotificationPriority::Normal,
                badge_count: None,
                sound: None,
                category_id: None,
                thread_id: None,
                trigger: NotificationTriggerValue::NotificationImmediateTrigger(
                    NotificationImmediateTriggerValue {
                        kind: "immediate".to_string(),
                    },
                ),
                action_id: None,
                data_json: None,
            },
        },
    })
}

/// Build one representative background event payload.
fn test_background_event() -> BackgroundEventValue {
    BackgroundEventValue::BackgroundTaskReadyEvent(BackgroundTaskReadyEventValue {
        kind: "taskReady".to_string(),
        metadata: BackgroundEventMetadataValue {
            timestamp_ns: 42,
            sequence: 7,
            identifier: "sync".to_string(),
            execution_id: "execution-1".to_string(),
            deadline_unix_ns: 0,
        },
    })
}

/// Build one representative location sample payload.
fn test_location_sample() -> LocationSampleValue {
    LocationSampleValue {
        latitude_degrees: 47.3769,
        longitude_degrees: 8.5417,
        altitude_meters: Some(408.0),
        horizontal_accuracy_meters: Some(5.0),
        vertical_accuracy_meters: Some(8.0),
        speed_meters_per_second: Some(0.0),
        heading_degrees: Some(0.0),
        timestamp_unix_ns: 42,
    }
}
