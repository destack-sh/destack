use std::sync::Arc;

use crate::host::abi::document::HostDocumentResult;
use crate::host::abi::intent::HostIntentEvent as HostAbiIntentEvent;
use crate::host::apple::ingress::lifecycle::host_lifecycle_state_for_application_lifecycle;
use crate::host::apple::ingress::{
    IosApplicationLifecycle, ios_notify_background_event, ios_notify_document_result,
    ios_notify_intent_event, ios_notify_location_sample, ios_notify_notification_event,
    ios_notify_permission_result,
};
use crate::host::core::{HostQueue, HostRequestId, HostSessionRegistry};
use crate::host::{
    HostBackgroundEvent, HostDocumentEvent, HostEvent, HostIntentEvent, HostIntentPayload,
    HostLifecycleState, HostLocationEvent, HostNotificationEvent, HostPermissionEvent, Platform,
};
use crate::platform::NativeAbiCodec;
use crate::platform::os::{
    BackgroundEventMetadataValue, BackgroundEventValue, BackgroundTaskReadyEventValue,
    DocumentDescriptorValue, IntentEventMetadataValue, IntentEventValue, IntentOpenUrlEventValue,
    IntentOpenUrlPayloadValue, LocationSampleValue, NotificationDeliveredEventValue,
    NotificationEventMetadataValue, NotificationEventValue, NotificationImmediateTriggerValue,
    NotificationPriority, NotificationRequestValue, NotificationTriggerValue, Permission,
};
use crate::runtime::BindingCallContext;

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
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();

    ios_notify_permission_result(runtime_id.0, 7, "camera", true).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            request_id: Some(HostRequestId(7)),
            permission: Permission::Camera,
            granted: true,
        })],
    );
}

#[test]
fn test_notify_intent_event_enqueues_intent_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
    let event = HostAbiIntentEvent::from_value(&BindingCallContext::default(), test_intent_event());

    ios_notify_intent_event(runtime_id.0, event).unwrap();

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

/// Build one representative intent event payload.
fn test_intent_event() -> IntentEventValue {
    IntentEventValue::IntentOpenUrlEvent(IntentOpenUrlEventValue {
        kind: "openUrl".to_string(),
        metadata: IntentEventMetadataValue {
            timestamp_ns: 42,
            sequence: 7,
            source: Some("com.example.source".to_string()),
        },
        payload: IntentOpenUrlPayloadValue {
            url: "https://example.com".to_string(),
        },
    })
}

#[test]
fn test_notify_notification_event_enqueues_notification_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
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
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
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
fn test_notify_document_result_enqueues_document_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
    let documents = vec![test_document_descriptor()];
    let result = HostDocumentResult {
        request_id: 7,
        documents: <_ as NativeAbiCodec>::from_value(
            &BindingCallContext::default(),
            documents.clone(),
        ),
    };

    ios_notify_document_result(runtime_id.0, result).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Document(Box::new(HostDocumentEvent {
            request_id: HostRequestId(7),
            documents,
        }))],
    );
}

#[test]
fn test_notify_location_sample_enqueues_location_event_for_runtime_bridge() {
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration =
        HostSessionRegistry::register_queue(Platform::IOS, runtime_id, Arc::clone(&queue), None);
    let runtime_id = registration.host_session_id();
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
        uri: "file:///tmp/fixture.txt".to_string(),
        local_path: None,
        name: "fixture.txt".to_string(),
        size_bytes: Some(12),
        mime_type: Some("text/plain".to_string()),
        is_directory: false,
        modified_unix_ns: Some(42),
    }
}
