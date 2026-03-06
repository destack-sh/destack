use destack_lsp_types as lsp;

use crate::lsp::{
    LspFixture, LspScenario, LspTestState, NormalizedDiagnostic, normalize_diagnostics,
    normalize_selection_ranges, normalize_workspace_diagnostic_partial_report,
    normalize_workspace_diagnostic_report, parse_expected_diagnostics_snapshot,
    parse_expected_selection_ranges_snapshot, verify_diagnostics, verify_exact_eq,
    verify_selection_ranges,
};

const INVALID_EXPORT_TEXT: &str = "export const value = ;\n";
const INVALID_DIAGNOSTIC_CODE: &str = "EP001";
const INVALID_DIAGNOSTIC_SOURCE: &str = "destack";
const INVALID_DIAGNOSTIC_MESSAGE: &str = "parse error: unexpected ; in Expression";

/// Run the diagnostics and selection-range lifecycle scenarios declared by one fixture.
pub(crate) fn run_diagnostic_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // document diagnostics for the initial open overlay
    if fixture.has_scenario(LspScenario::DocumentDiagnosticOpenOverlay) {
        let diag_marker = fixture
            .marker("diag")
            .ok_or_else(|| "diagnostic fixture is missing /*diag*/ marker".to_string())?;
        let semicolon_range = fixture
            .range_by_text(";")
            .ok_or_else(|| "diagnostic fixture is missing [|;|] expected range".to_string())?;
        let expected_diagnostics = vec![diagnostic_from_range(
            &diag_marker.file_path,
            semicolon_range.start_line,
            semicolon_range.start_character,
            semicolon_range.end_line,
            semicolon_range.end_character,
        )];

        let report = test_state.request_document_diagnostics(&diag_marker.file_path)?;
        let actual_diagnostics = normalize_document_diagnostics(&diag_marker.file_path, report)?;

        verify_diagnostics(&actual_diagnostics, &expected_diagnostics)?;
        test_state.verify().error_exists_at_range(
            semicolon_range,
            INVALID_DIAGNOSTIC_CODE,
            Some(INVALID_DIAGNOSTIC_MESSAGE),
        )?;
    }

    // marker-window diagnostic helpers should observe the same exact parse error
    if fixture.has_scenario(LspScenario::DiagnosticMarkerWindows) {
        let file_path = fixture
            .marker("error_before")
            .map(|marker| marker.file_path.clone())
            .ok_or_else(|| "diagnostic marker fixture is missing /*error_before*/".to_string())?;
        let report = test_state.request_document_diagnostics(&file_path)?;
        let actual_diagnostics = normalize_document_diagnostics(&file_path, report)?;

        verify_exact_eq("diagnostic count", &actual_diagnostics.len(), &1)?;
        test_state
            .verify()
            .error_exists_between_markers("error_start", "error_end")?;
        test_state
            .verify()
            .error_exists_after_marker(Some("error_before"))?;
        test_state
            .verify()
            .error_exists_before_marker(Some("error_after"))?;
    }

    // unsaved overlay change updates diagnostics without touching disk content
    if fixture.has_scenario(LspScenario::DocumentDiagnosticChangeOverlay) {
        let file_path = fixture
            .marker("change")
            .map(|marker| marker.file_path.clone())
            .ok_or_else(|| "change fixture is missing /*change*/ marker".to_string())?;
        let expected_diagnostics = expected_invalid_export_diagnostics(&file_path)?;

        test_state.replace_document_text(&file_path, INVALID_EXPORT_TEXT)?;
        let pushed_diagnostics = normalize_diagnostics(
            &file_path,
            &test_state
                .wait_for_diagnostics_version(&file_path, 2)?
                .diagnostics,
        );
        verify_diagnostics(&pushed_diagnostics, &expected_diagnostics)?;
        let report = test_state.request_document_diagnostics(&file_path)?;
        let actual_diagnostics = normalize_document_diagnostics(&file_path, report)?;

        verify_diagnostics(&actual_diagnostics, &expected_diagnostics)?;
    }

    // save persists the overlay to disk so later pulls still report the saved diagnostics
    if fixture.has_scenario(LspScenario::DocumentDiagnosticSavePersistsOverlay) {
        let file_path = fixture
            .marker("save")
            .map(|marker| marker.file_path.clone())
            .ok_or_else(|| "save fixture is missing /*save*/ marker".to_string())?;
        let expected_diagnostics = expected_invalid_export_diagnostics(&file_path)?;

        test_state.replace_document_text(&file_path, INVALID_EXPORT_TEXT)?;
        let pushed_diagnostics = normalize_diagnostics(
            &file_path,
            &test_state
                .wait_for_diagnostics_version(&file_path, 2)?
                .diagnostics,
        );
        verify_diagnostics(&pushed_diagnostics, &expected_diagnostics)?;
        test_state.save_file(&file_path)?;
        test_state.close_file(&file_path)?;
        let report = test_state.request_document_diagnostics(&file_path)?;
        let actual_diagnostics = normalize_document_diagnostics(&file_path, report)?;

        verify_diagnostics(&actual_diagnostics, &expected_diagnostics)?;
    }

    // close drops the unsaved overlay so later pulls fall back to clean disk content
    if fixture.has_scenario(LspScenario::DocumentDiagnosticCloseRevertsOverlay) {
        let file_path = fixture
            .marker("close")
            .map(|marker| marker.file_path.clone())
            .ok_or_else(|| "close fixture is missing /*close*/ marker".to_string())?;
        let expected_diagnostics = expected_invalid_export_diagnostics(&file_path)?;

        test_state.replace_document_text(&file_path, INVALID_EXPORT_TEXT)?;
        let pushed_diagnostics = normalize_diagnostics(
            &file_path,
            &test_state
                .wait_for_diagnostics_version(&file_path, 2)?
                .diagnostics,
        );
        verify_diagnostics(&pushed_diagnostics, &expected_diagnostics)?;
        test_state.close_file(&file_path)?;
        let closed_diagnostics = normalize_diagnostics(
            &file_path,
            &test_state
                .wait_for_diagnostics_version(&file_path, 2)?
                .diagnostics,
        );
        let report = test_state.request_document_diagnostics(&file_path)?;
        let actual_diagnostics = normalize_document_diagnostics(&file_path, report)?;

        verify_diagnostics(&closed_diagnostics, &[])?;
        verify_diagnostics(&actual_diagnostics, &[])?;
    }

    // full workspace diagnostics
    if let Some(expected_snapshot) = fixture.expectations.diagnostics.workspace_text.as_deref() {
        let expected_diagnostics = parse_expected_diagnostics_snapshot(&expected_snapshot)?;

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
        let expected_diagnostics = parse_expected_diagnostics_snapshot(&expected_snapshot)?;
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

    // selection ranges should stream exact partial results
    if let Some(expected_snapshot) = fixture
        .expectations
        .diagnostics
        .selection_range_text
        .as_deref()
    {
        let expected_ranges = parse_expected_selection_ranges_snapshot(&expected_snapshot)?;
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

    // clean documents should report no diagnostics through the exact verify surface
    if fixture.has_scenario(LspScenario::NoErrorsCleanWorkspace) {
        let file_path = fixture
            .files
            .iter()
            .next()
            .map(|file| file.path.clone())
            .ok_or_else(|| "no-errors fixture is missing a file".to_string())?;
        test_state.go_to().file(&file_path)?;

        test_state.verify().no_errors()?;
        test_state.verify().number_of_errors_in_current_file(0)?;
    }

    Ok(())
}

/// Normalize one document diagnostics response into an exact list.
fn normalize_document_diagnostics(
    file_path: &str,
    report: lsp::DocumentDiagnosticReportResult,
) -> Result<Vec<NormalizedDiagnostic>, String> {
    let lsp::DocumentDiagnosticReportResult::Report(report) = report else {
        return Err("expected document diagnostic report".to_string());
    };
    let lsp::DocumentDiagnosticReport::Full(full) = report else {
        return Err("expected full document diagnostics".to_string());
    };

    Ok(normalize_diagnostics(
        file_path,
        &full.full_document_diagnostic_report.items,
    ))
}

/// Build the canonical exact invalid export diagnostic for one file.
fn expected_invalid_export_diagnostics(
    file_path: &str,
) -> Result<Vec<NormalizedDiagnostic>, String> {
    let semicolon = INVALID_EXPORT_TEXT
        .find(';')
        .ok_or_else(|| "invalid export fixture text is missing ';'".to_string())?;

    Ok(vec![diagnostic_from_range(
        file_path,
        0,
        semicolon,
        0,
        semicolon + 1,
    )])
}

/// Build one normalized diagnostic from explicit range coordinates.
fn diagnostic_from_range(
    file_path: &str,
    start_line: usize,
    start_character: usize,
    end_line: usize,
    end_character: usize,
) -> NormalizedDiagnostic {
    NormalizedDiagnostic {
        file_path: file_path.to_string(),
        start_line,
        start_character,
        end_line,
        end_character,
        severity: Some("error"),
        code: Some(INVALID_DIAGNOSTIC_CODE.to_string()),
        source: Some(INVALID_DIAGNOSTIC_SOURCE.to_string()),
        message: INVALID_DIAGNOSTIC_MESSAGE.to_string(),
    }
}
