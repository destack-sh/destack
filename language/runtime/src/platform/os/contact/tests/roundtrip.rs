use super::common::{
    contact_draft_harness_value, contact_query_harness_value, decode_contact, decode_contact_page,
    decode_string_value, desktop_contact_test_state, sample_contact, sample_contact_draft,
    sample_contact_page, sample_contact_query, string_harness_value, with_desktop_contact_context,
};

/// Verify desktop contact operations route through the real runtime surface.
#[test]
fn test_contact_roundtrip_routes_through_desktop_host_lane() {
    with_desktop_contact_context(|mut context| {
        // reset one shared mock state for this harness pass
        {
            let mut state = desktop_contact_test_state()
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            *state = Default::default();
        }

        // contact list and search
        let query = contact_query_harness_value(&mut context, sample_contact_query())?;
        let page = context.destack_os_contact_list(query)?;
        let page = decode_contact_page(&mut context, page)?;
        let search_text = string_harness_value(&mut context, "Ada");
        let search_query = contact_query_harness_value(&mut context, sample_contact_query())?;
        let search_page = context.destack_os_contact_search(search_text, search_query)?;
        let search_page = decode_contact_page(&mut context, search_page)?;

        assert_eq!(page, sample_contact_page());
        assert_eq!(search_page, sample_contact_page());

        // contact read and create
        let read_id = string_harness_value(&mut context, "contact-1");
        let contact = context.destack_os_contact_read(read_id)?;
        let contact = decode_contact(&mut context, contact)?;
        let create_draft = contact_draft_harness_value(&mut context, sample_contact_draft("Grace"));
        let created_id = context.destack_os_contact_create(create_draft)?;
        let created_id = decode_string_value(&mut context, created_id)?;

        assert_eq!(contact, sample_contact());
        assert_eq!(created_id, "contact-created");

        // update and delete
        let update_id = string_harness_value(&mut context, "contact-1");
        let update_draft = contact_draft_harness_value(&mut context, sample_contact_draft("Ada"));
        context.destack_os_contact_update(update_id, update_draft)?;

        let delete_id = string_harness_value(&mut context, "contact-1");
        context.destack_os_contact_delete(delete_id)?;

        // recorded backend state
        let state = desktop_contact_test_state()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        assert_eq!(state.list_queries, vec![sample_contact_query()]);
        assert_eq!(
            state.search_queries,
            vec![("Ada".to_string(), sample_contact_query())]
        );
        assert_eq!(state.read_ids, vec!["contact-1".to_string()]);
        assert_eq!(state.created_contacts, vec![sample_contact_draft("Grace")]);
        assert_eq!(
            state.updated_contacts,
            vec![("contact-1".to_string(), sample_contact_draft("Ada"))]
        );
        assert_eq!(state.deleted_ids, vec!["contact-1".to_string()]);

        Ok(())
    })
    .expect("desktop contact roundtrip should succeed")
}
