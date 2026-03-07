use destack_lsp_types as lsp;

use crate::lsp::{
    LspFixture, LspTestState, normalize_selection_ranges,
    normalize_workspace_diagnostic_partial_report, normalize_workspace_diagnostic_report,
    parse_expected_diagnostics_snapshot, parse_expected_selection_ranges_snapshot,
    verify_diagnostics, verify_selection_ranges,
};

/// Run the diagnostic and selection-range cases declared by one fixture.
pub(crate) fn run_diagnostic_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // full workspace diagnostics
    if let Some(expected_snapshot) = fixture.expectations.diagnostics.workspace_text.as_deref() {
        let expected_diagnostics = parse_expected_diagnostics_snapshot(expected_snapshot)?;

        let report = test_state.request_workspace_diagnostics()?;
        let actual_diagnostics =
            normalize_workspace_diagnostic_report(test_state.workspace_root(), &report)?;

        verify_diagnostics(&actual_diagnostics, &expected_diagnostics)?;
    }

    // partial workspace diagnostics
    if let Some(expected_snapshot) = fixture
        .expectations
        .diagnostics
        .workspace_partial_text
        .as_deref()
    {
        let expected_diagnostics = parse_expected_diagnostics_snapshot(expected_snapshot)?;
        let partial_result_token = lsp::ProgressToken::Number(1);
        let work_done_token = lsp::ProgressToken::String("workspace-diagnostic".to_string());

        let request_id = test_state.start_workspace_diagnostics_with_partial_progress(
            partial_result_token.clone(),
            Some(work_done_token.clone()),
        )?;
        test_state.wait_for_work_done_progress_kind(&work_done_token, "begin")?;

        let progress = test_state.wait_for_partial_progress(&partial_result_token)?;
        let partial_report: lsp::WorkspaceDiagnosticReportPartialResult =
            serde_json::from_value(progress).map_err(|error| {
                format!("failed to decode workspace partial diagnostics: {error}")
            })?;
        let actual_diagnostics = normalize_workspace_diagnostic_partial_report(
            test_state.workspace_root(),
            &partial_report,
        )?;

        verify_diagnostics(&actual_diagnostics, &expected_diagnostics)?;

        test_state.wait_for_work_done_progress_kind(&work_done_token, "end")?;
        let response = test_state.await_request(request_id)?;
        if !response.is_ok() {
            return Err(format!(
                "workspace diagnostic partial request failed: {:#?}",
                response.error()
            ));
        }
    }

    // selection range partial results
    if let Some(expected_snapshot) = fixture
        .expectations
        .diagnostics
        .selection_range_text
        .as_deref()
    {
        let expected_ranges = parse_expected_selection_ranges_snapshot(expected_snapshot)?;
        let partial_result_token = lsp::ProgressToken::Number(7);

        test_state.go_to_marker("selection")?;
        let request_id = test_state
            .start_selection_ranges_with_partial_progress(partial_result_token.clone())?;
        let progress = test_state.wait_for_partial_progress(&partial_result_token)?;
        let ranges: Vec<lsp::SelectionRange> = serde_json::from_value(progress)
            .map_err(|error| format!("failed to decode selection ranges: {error}"))?;
        let active_file_path = test_state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "selection range request did not keep an active file".to_string())?;
        let actual_ranges = normalize_selection_ranges(&active_file_path, &ranges);

        verify_selection_ranges(&actual_ranges, &expected_ranges)?;

        let response = test_state.await_request(request_id)?;
        if !response.is_ok() {
            return Err(format!(
                "selection range partial request failed: {:#?}",
                response.error()
            ));
        }
    }

    Ok(())
}
