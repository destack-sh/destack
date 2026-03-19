use super::common::{
    decode_notification_event_value, notification_event_metadata,
    notification_event_open_options_with_flags, notification_post_request,
    with_notification_context,
};
use crate::platform::os::abi_generated::{
    NotificationDeliveredEventValue, NotificationDismissedEventValue, NotificationEventValue,
    NotificationInteractedEventValue, NotificationInteractedPayloadValue,
};

/// Exercise notification event stream filtering for interacted and dismissed events.
#[test]
fn test_notification_event_stream_filters_interacted_and_dismissed_events() {
    with_notification_context(|mut context| {
        // open one stream that only accepts interacted events
        let interacted_options =
            notification_event_open_options_with_flags(&context, false, true, false);
        let interacted_handle = context.destack_os_notification_event_open(interacted_options)?;

        // enqueue one delivered event that should be ignored by the stream
        context.enqueue_notification_event(NotificationEventValue::NotificationDeliveredEvent(
            NotificationDeliveredEventValue {
                kind: "delivered".to_string(),
                metadata: notification_event_metadata(
                    "delivered-1",
                    notification_post_request(),
                    1,
                ),
            },
        ))?;

        // enqueue one interacted event that should be observed
        context.enqueue_notification_event(NotificationEventValue::NotificationInteractedEvent(
            NotificationInteractedEventValue {
                kind: "interacted".to_string(),
                metadata: notification_event_metadata(
                    "interacted-1",
                    notification_post_request(),
                    2,
                ),
                payload: NotificationInteractedPayloadValue {
                    action_id: Some("reply".to_string()),
                    action_response_text: Some("hello".to_string()),
                },
            },
        ))?;

        // read back the interacted event and verify the payload survived
        let interacted_event =
            context.destack_os_notification_event_read(interacted_handle, 1_000_000)?;
        let interacted_event = decode_notification_event_value(&mut context, interacted_event)?;

        match interacted_event {
            NotificationEventValue::NotificationInteractedEvent(event) => {
                assert_eq!(event.metadata.id, "interacted-1");
                assert_eq!(event.payload.action_id.as_deref(), Some("reply"));
                assert_eq!(event.payload.action_response_text.as_deref(), Some("hello"));
            }
            _ => panic!("expected one interacted notification event"),
        }

        context.destack_os_notification_event_close(interacted_handle)?;

        // open one stream that only accepts dismissed events
        let dismissed_options =
            notification_event_open_options_with_flags(&context, false, false, true);
        let dismissed_handle = context.destack_os_notification_event_open(dismissed_options)?;

        // enqueue one dismissed event after the stream is open
        context.enqueue_notification_event(NotificationEventValue::NotificationDismissedEvent(
            NotificationDismissedEventValue {
                kind: "dismissed".to_string(),
                metadata: notification_event_metadata(
                    "dismissed-1",
                    notification_post_request(),
                    3,
                ),
            },
        ))?;

        // read back the dismissed event and verify it was routed through the stream filter
        let dismissed_event =
            context.destack_os_notification_event_read(dismissed_handle, 1_000_000)?;
        let dismissed_event = decode_notification_event_value(&mut context, dismissed_event)?;

        match dismissed_event {
            NotificationEventValue::NotificationDismissedEvent(event) => {
                assert_eq!(event.metadata.id, "dismissed-1");
                assert_eq!(event.metadata.sequence, 3);
            }
            _ => panic!("expected one dismissed notification event"),
        }

        context.destack_os_notification_event_close(dismissed_handle)?;

        Ok(())
    })
    .expect("notification event stream filtering should succeed")
}
