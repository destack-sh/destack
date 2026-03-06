use crate::lsp::{
    LspFixture, LspTestState, normalize_call_hierarchy_incoming, normalize_call_hierarchy_outgoing,
    normalize_type_hierarchy_items, parse_expected_call_hierarchy_calls,
    parse_expected_type_hierarchy_items, verify_call_hierarchy_calls, verify_type_hierarchy_items,
};

/// Run the hierarchy scenarios declared by one fixture.
pub(crate) fn run_hierarchy_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // call hierarchy incoming
    if let Some(expected_snapshot) = fixture.expectations.hierarchy.call_incoming_text.as_deref() {
        let expected_calls = parse_expected_call_hierarchy_calls(&expected_snapshot)?;

        test_state.go_to_marker("call_hierarchy")?;
        let prepared_items = test_state
            .request_prepare_call_hierarchy()?
            .ok_or_else(|| "expected call hierarchy prepare result".to_string())?;
        let prepared_item = prepared_items
            .into_iter()
            .next()
            .ok_or_else(|| "expected one call hierarchy prepare item".to_string())?;
        let incoming_calls = test_state
            .request_call_hierarchy_incoming(prepared_item)?
            .unwrap_or_default();
        let actual_calls =
            normalize_call_hierarchy_incoming(test_state.workspace_root(), &incoming_calls)?;

        verify_call_hierarchy_calls(&actual_calls, &expected_calls)?;
    }

    // call hierarchy outgoing
    if let Some(expected_snapshot) = fixture.expectations.hierarchy.call_outgoing_text.as_deref() {
        let expected_calls = parse_expected_call_hierarchy_calls(&expected_snapshot)?;

        test_state.go_to_marker("call_hierarchy")?;
        let prepared_items = test_state
            .request_prepare_call_hierarchy()?
            .ok_or_else(|| "expected call hierarchy prepare result".to_string())?;
        let prepared_item = prepared_items
            .into_iter()
            .next()
            .ok_or_else(|| "expected one call hierarchy prepare item".to_string())?;
        let outgoing_calls = test_state
            .request_call_hierarchy_outgoing(prepared_item)?
            .unwrap_or_default();
        let actual_calls =
            normalize_call_hierarchy_outgoing(test_state.workspace_root(), &outgoing_calls)?;

        verify_call_hierarchy_calls(&actual_calls, &expected_calls)?;
    }

    // type hierarchy supertypes
    if let Some(expected_snapshot) = fixture
        .expectations
        .hierarchy
        .type_supertypes_text
        .as_deref()
    {
        let expected_items = parse_expected_type_hierarchy_items(&expected_snapshot)?;

        test_state.go_to_marker("type_hierarchy")?;
        let prepared_items = test_state
            .request_prepare_type_hierarchy()?
            .ok_or_else(|| "expected type hierarchy prepare result".to_string())?;
        let prepared_item = prepared_items
            .into_iter()
            .next()
            .ok_or_else(|| "expected one type hierarchy prepare item".to_string())?;
        let actual_items = test_state
            .request_type_hierarchy_supertypes(prepared_item)?
            .unwrap_or_default();
        let actual_items =
            normalize_type_hierarchy_items(test_state.workspace_root(), &actual_items)?;

        verify_type_hierarchy_items(&actual_items, &expected_items)?;
    }

    // type hierarchy subtypes
    if let Some(expected_snapshot) = fixture.expectations.hierarchy.type_subtypes_text.as_deref() {
        let expected_items = parse_expected_type_hierarchy_items(&expected_snapshot)?;

        test_state.go_to_marker("type_hierarchy")?;
        let prepared_items = test_state
            .request_prepare_type_hierarchy()?
            .ok_or_else(|| "expected type hierarchy prepare result".to_string())?;
        let prepared_item = prepared_items
            .into_iter()
            .next()
            .ok_or_else(|| "expected one type hierarchy prepare item".to_string())?;
        let actual_items = test_state
            .request_type_hierarchy_subtypes(prepared_item)?
            .unwrap_or_default();
        let actual_items =
            normalize_type_hierarchy_items(test_state.workspace_root(), &actual_items)?;

        verify_type_hierarchy_items(&actual_items, &expected_items)?;
    }

    Ok(())
}
