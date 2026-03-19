use super::common::{
    decode_notification_event, decode_notification_id, notification_event_open_options,
    notification_post_request, notification_request_harness_value, with_notification_context,
};

/// Exercise immediate delivery events through the desktop notification host lane.
#[test]
fn test_notification_post_emits_delivered_event() {
    with_notification_context(|mut context| {
        // open one delivered-event stream first so the notification is observed
        let options = notification_event_open_options(&context);
        let handle = context.destack_os_notification_event_open(options)?;

        // post one immediate notification through the desktop host lane
        let request = notification_request_harness_value(&mut context, notification_post_request());
        let id = context.destack_os_notification_post(request)?;
        let id = decode_notification_id(&mut context, id)?;

        // read back the delivered event and verify the request metadata
        let event = context.destack_os_notification_event_read(handle, 1_000_000)?;
        let event = decode_notification_event(&mut context, event)?;

        assert_eq!(event.metadata.id, id);
        assert_eq!(event.metadata.request.tag, "welcome");
        assert_eq!(event.metadata.request.title, "Welcome");

        context.destack_os_notification_event_close(handle)?;

        Ok(())
    })
    .expect("notification delivery should emit one delivered event")
}
