use super::common::{
    calendar_draft_harness_value, calendar_query_harness_value, decode_calendar_descriptors,
    decode_calendar_event, decode_calendar_events, decode_string_value,
    desktop_calendar_test_state, sample_calendar_descriptor, sample_calendar_draft,
    sample_calendar_event, sample_calendar_query, string_harness_value,
    with_desktop_calendar_context,
};

/// Verify desktop calendar operations route through the real runtime surface.
#[test]
fn test_calendar_roundtrip_routes_through_desktop_host_lane() {
    with_desktop_calendar_context(|mut context| {
        // reset one shared mock state for this harness pass
        {
            let mut state = desktop_calendar_test_state()
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            *state = Default::default();
        }

        // calendar list
        let calendars = context.destack_os_calendar_list()?;
        let calendars = decode_calendar_descriptors(&mut context, calendars)?;

        assert_eq!(calendars, vec![sample_calendar_descriptor()]);

        // event list and read
        let query = calendar_query_harness_value(&mut context, sample_calendar_query())?;
        let events = context.destack_os_calendar_event_list(query)?;
        let events = decode_calendar_events(&mut context, events)?;
        let read_id = string_harness_value(&mut context, "event-1");
        let event = context.destack_os_calendar_event_read(read_id)?;
        let event = decode_calendar_event(&mut context, event)?;

        assert_eq!(events, vec![sample_calendar_event()]);
        assert_eq!(event, sample_calendar_event());

        // create, update, and delete
        let create_draft =
            calendar_draft_harness_value(&mut context, sample_calendar_draft("Draft create"));
        let created_id = context.destack_os_calendar_event_create(create_draft)?;
        let created_id = decode_string_value(&mut context, created_id)?;

        let update_id = string_harness_value(&mut context, "event-1");
        let update_draft =
            calendar_draft_harness_value(&mut context, sample_calendar_draft("Draft update"));
        context.destack_os_calendar_event_update(update_id, update_draft)?;

        let delete_id = string_harness_value(&mut context, "event-1");
        context.destack_os_calendar_event_delete(delete_id)?;

        assert_eq!(created_id, "event-created");

        // recorded backend state
        let state = desktop_calendar_test_state()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        assert_eq!(state.event_queries, vec![sample_calendar_query()]);
        assert_eq!(state.read_ids, vec!["event-1".to_string()]);
        assert_eq!(
            state.created_events,
            vec![sample_calendar_draft("Draft create")]
        );
        assert_eq!(
            state.updated_events,
            vec![("event-1".to_string(), sample_calendar_draft("Draft update"))]
        );
        assert_eq!(state.deleted_ids, vec!["event-1".to_string()]);

        Ok(())
    })
    .expect("desktop calendar roundtrip should succeed")
}
