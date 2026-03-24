use crate::host::android::ingress::{
    android_notify_background_event, android_notify_document_result,
    android_notify_intent_open_url, android_notify_location_sample,
    android_notify_notification_event,
};
use crate::host::android::tests::register_android_runtime;
use crate::host::core::HostRequestId;
use crate::host::{
    HostBackgroundEvent, HostDocumentEvent, HostEvent, HostIntentEvent, HostIntentPayload,
    HostLocationEvent, HostNotificationEvent,
};
use crate::platform::os::{
    BackgroundEventMetadataValue, BackgroundEventValue, BackgroundTaskReadyEventValue,
    DocumentDescriptorValue, LocationSampleValue, NotificationDeliveredEventValue,
    NotificationEventMetadataValue, NotificationEventValue, NotificationImmediateTriggerValue,
    NotificationPriority, NotificationRequestValue, NotificationTriggerValue,
};

/// Enqueue one Android intent event for the registered runtime.
#[test]
fn test_notify_intent_open_url_enqueues_intent_event_for_runtime_bridge() {
    let (queue, _registration, runtime_id) = register_android_runtime();

    android_notify_intent_open_url(
        runtime_id,
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

/// Enqueue one Android notification event for the registered runtime.
#[test]
fn test_notify_notification_event_enqueues_notification_event_for_runtime_bridge() {
    let (queue, _registration, runtime_id) = register_android_runtime();
    let event = test_notification_event();

    android_notify_notification_event(runtime_id, event.clone()).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Notification(Box::new(HostNotificationEvent {
            event,
        }))],
    );
}

/// Enqueue one Android background event for the registered runtime.
#[test]
fn test_notify_background_event_enqueues_background_event_for_runtime_bridge() {
    let (queue, _registration, runtime_id) = register_android_runtime();
    let event = test_background_event();

    android_notify_background_event(runtime_id, event.clone()).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Background(Box::new(HostBackgroundEvent {
            event
        }))],
    );
}

/// Enqueue one Android document event for the registered runtime.
#[test]
fn test_notify_document_result_enqueues_document_event_for_runtime_bridge() {
    let (queue, _registration, runtime_id) = register_android_runtime();
    let documents = vec![test_document_descriptor()];

    android_notify_document_result(runtime_id, 7, documents.clone()).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Document(Box::new(HostDocumentEvent {
            request_id: HostRequestId(7),
            documents,
        }))],
    );
}

/// Enqueue one Android location event for the registered runtime.
#[test]
fn test_notify_location_sample_enqueues_location_event_for_runtime_bridge() {
    let (queue, _registration, runtime_id) = register_android_runtime();
    let sample = test_location_sample();

    android_notify_location_sample(runtime_id, "watch-1", sample).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Location(Box::new(HostLocationEvent {
            watch_id: "watch-1".to_string(),
            sample,
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
        altitude_meters: 408.0,
        horizontal_accuracy_meters: 5.0,
        vertical_accuracy_meters: 8.0,
        speed_meters_per_second: 0.0,
        heading_degrees: 0.0,
        timestamp_unix_ns: 42,
    }
}

/// Build one representative document descriptor payload.
fn test_document_descriptor() -> DocumentDescriptorValue {
    DocumentDescriptorValue {
        uri: "content://destack/document/1".to_string(),
        local_path: None,
        name: "fixture.txt".to_string(),
        size_bytes: Some(12),
        mime_type: Some("text/plain".to_string()),
        is_directory: false,
        modified_unix_ns: Some(42),
    }
}
