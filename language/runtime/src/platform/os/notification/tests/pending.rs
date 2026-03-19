use super::common::{
    decode_notification_id, decode_notification_pending_list, notification_request_harness_value,
    notification_schedule_request, string_harness_value, with_notification_context,
};

/// Exercise pending notification scheduling and cancellation through both harnesses.
#[test]
fn test_notification_schedule_pending_roundtrip() {
    with_notification_context(|mut context| {
        // schedule one deferred notification and read back its identifier
        let request =
            notification_request_harness_value(&mut context, notification_schedule_request());
        let id = context.destack_os_notification_schedule(request)?;
        let id = decode_notification_id(&mut context, id)?;

        // verify the scheduled descriptor is visible through pendingList
        let pending = context.destack_os_notification_pending_list()?;
        let pending = decode_notification_pending_list(&mut context, pending)?;

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].request.tag, "sync");

        // cancel the pending descriptor and verify it disappears
        let id = string_harness_value(&mut context, pending[0].id.as_str());
        context.destack_os_notification_pending_cancel(id)?;

        let pending = context.destack_os_notification_pending_list()?;
        let pending = decode_notification_pending_list(&mut context, pending)?;

        assert!(pending.is_empty());

        Ok(())
    })
    .expect("notification pending roundtrip should succeed")
}
