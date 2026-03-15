use crate::host::android::tests::register_android_runtime;
use crate::host::{HostEvent, HostIntentEvent, HostIntentPayload, android_notify_intent_open_url};

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
