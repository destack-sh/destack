use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;

use crate::lsp::runner::expected_location_from_marker;
use crate::lsp::{
    LspFixture, LspScenario, LspTestState, normalize_definition_response, normalize_diagnostics,
    normalize_workspace_diagnostic_report, verify_definition_locations, verify_diagnostics,
    verify_exact_eq,
};

/// Run the edit and mixed edit or diagnostic scenarios declared by one fixture.
pub(crate) fn run_edit_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // interleaved multi-file edits should keep diagnostics versions and definitions stable
    if fixture.has_scenario(LspScenario::MultifileEditResponsiveness) {
        let definition_marker = fixture
            .marker("def")
            .ok_or_else(|| "multi-file fixture is missing /*def*/ marker".to_string())?;
        let use_marker = fixture
            .marker("use")
            .ok_or_else(|| "multi-file fixture is missing /*use*/ marker".to_string())?;
        let lib_file_path = definition_marker.file_path.clone();
        let main_file_path = use_marker.file_path.clone();
        let expected_definition = expected_location_from_marker(fixture, definition_marker)?;
        let expected_workspace_texts = vec![lib_file_path.clone(), main_file_path.clone()];

        for iteration in 0..4 {
            let next_version = iteration + 2;
            let next_lib_value = iteration + 2;
            let next_main_value = iteration + 3;
            let lib_text = format!("export const value: number = {next_lib_value};\n");
            let main_text = format!(
                "import {{ value }} from \"./lib.ds\";\nconst output: number = value + {next_main_value};\nexport {{ output }};\n"
            );

            test_state.replace_document_text(&lib_file_path, &lib_text)?;
            test_state.replace_document_text(&main_file_path, &main_text)?;

            let lib_diagnostics =
                test_state.wait_for_diagnostics_version(&lib_file_path, next_version)?;
            let main_diagnostics =
                test_state.wait_for_diagnostics_version(&main_file_path, next_version)?;

            verify_exact_eq(
                "lib diagnostics version",
                &lib_diagnostics.version,
                &Some(next_version),
            )?;
            verify_exact_eq(
                "main diagnostics version",
                &main_diagnostics.version,
                &Some(next_version),
            )?;
            verify_diagnostics(
                &normalize_diagnostics(&lib_file_path, &lib_diagnostics.diagnostics),
                &[],
            )?;
            verify_diagnostics(
                &normalize_diagnostics(&main_file_path, &main_diagnostics.diagnostics),
                &[],
            )?;

            let workspace_report = test_state.request_workspace_diagnostics()?;
            let actual_workspace_texts = workspace_diagnostic_report_file_paths(
                test_state.workspace_root(),
                &workspace_report,
            )?;
            let actual_workspace_diagnostics = normalize_workspace_diagnostic_report(
                test_state.workspace_root(),
                &workspace_report,
            )?;

            verify_exact_eq(
                "workspace diagnostic files",
                &actual_workspace_texts,
                &expected_workspace_texts,
            )?;
            verify_diagnostics(&actual_workspace_diagnostics, &[])?;

            test_state.go_to_marker("use")?;
            let definition = test_state.request_definition()?.ok_or_else(|| {
                "expected goto definition result after multi-file edits".to_string()
            })?;
            let actual_definition =
                normalize_definition_response(test_state.workspace_root(), &definition)?;

            verify_definition_locations(
                &actual_definition,
                std::slice::from_ref(&expected_definition),
            )?;
        }
    }

    // native editor verbs should roundtrip to the expected current file text
    if fixture.has_scenario(LspScenario::EditRoundtrip) {
        let expected_text = fixture
            .expectations
            .edits
            .current_file_text
            .as_deref()
            .ok_or_else(|| {
                "edit roundtrip fixture is missing an lsp current_file block".to_string()
            })?;
        let source_file_path = fixture.first_file_path()?;

        test_state.go_to().file(&source_file_path)?;
        test_state.go_to().marker("edit")?;
        test_state.edit().move_left(2)?;
        test_state.edit().insert("!")?;
        test_state.edit().backspace(1)?;
        test_state.go_to().select("select_start", "select_end")?;
        test_state.edit().replace_selection("y")?;
        let caret = test_state.edit().caret_position()?;

        verify_exact_eq("caret offset", &caret.offset, &test_state.caret_offset())?;

        test_state
            .verify()
            .current_line_content_is("const message = \"hey\";")?;
        test_state.verify().current_file_content_is(expected_text)?;
        test_state.verify().text_at_caret_is("\";")?;
        test_state.verify().caret_at_marker(Some("after_edit"))?;
    }

    // select-all replacement should roundtrip through the active file surface
    if fixture.has_scenario(LspScenario::EditSelectAllReplace) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.go_to().select_all_in_file(&source_file_path)?;
        test_state.edit().replace_selection(&expected_text)?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // direct replace and multi-line insertions should roundtrip through the active file
    if fixture.has_scenario(LspScenario::EditReplaceAndInsertLines) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;
        let name_marker = fixture
            .marker("replace_name")
            .ok_or_else(|| "edit fixture is missing /*replace_name*/ marker".to_string())?;
        let number_marker = fixture
            .marker("replace_number")
            .ok_or_else(|| "edit fixture is missing /*replace_number*/ marker".to_string())?;

        test_state.go_to().file(&source_file_path)?;
        test_state
            .go_to()
            .position(name_marker.offset, Some(&source_file_path))?;
        test_state.verify().text_at_caret_is("alpha")?;

        let caret_offset = test_state.caret_offset();
        test_state
            .edit()
            .replace(caret_offset, "alpha".len(), "gamma")?;
        test_state
            .go_to()
            .position(number_marker.offset, Some(&source_file_path))?;
        let caret_offset = test_state.caret_offset();
        test_state.edit().replace(caret_offset, 1, "3")?;
        test_state.go_to().eof()?;
        test_state
            .edit()
            .insert_lines(["const delta = 4;", "const epsilon = 5;", ""])?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // range-driven selections should replace the selected text exactly
    if fixture.has_scenario(LspScenario::EditSelectRangeReplace) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;
        let beta_range = fixture
            .range_by_text("beta")
            .cloned()
            .ok_or_else(|| "edit fixture is missing [|beta|] range".to_string())?;

        test_state.go_to().file(&source_file_path)?;
        test_state.go_to().select_range(&beta_range)?;
        test_state.edit().replace_selection("omega")?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // bof, move-right, delete-at-caret, paste, and insert-line should compose exactly
    if fixture.has_scenario(LspScenario::EditPasteDeleteAndBof) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;
        let caret_marker = fixture
            .marker("caret")
            .ok_or_else(|| "edit fixture is missing /*caret*/ marker".to_string())?;

        test_state.go_to().file(&source_file_path)?;
        test_state.go_to().bof()?;
        test_state.edit().move_right(caret_marker.offset)?;
        test_state.edit().delete_at_caret(1)?;
        test_state.edit().paste("m")?;
        test_state.go_to().bof()?;
        test_state.edit().insert_line("// header")?;
        test_state.go_to().eof()?;
        test_state.edit().paste("const tail = 1;\n")?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // line-oriented edit helpers should rewrite the full file exactly
    if fixture.has_scenario(LspScenario::EditDeleteLine) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.edit().delete_line(1)?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // inclusive line-range deletion should rewrite the full file exactly
    if fixture.has_scenario(LspScenario::EditDeleteLineRange) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.edit().delete_line_range(1, 2)?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    // line replacement should preserve newline shape and rewrite the full file exactly
    if fixture.has_scenario(LspScenario::EditReplaceLine) {
        let (source_file_path, expected_text) = edit_expected_text(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.edit().replace_line(1, "const beta = 20;")?;

        test_state
            .verify()
            .current_file_content_is(&expected_text)?;
    }

    Ok(())
}

/// Return the primary file and expected text for one edit fixture.
fn edit_expected_text(fixture: &LspFixture) -> Result<(String, String), String> {
    let expected_text = fixture
        .expectations
        .edits
        .current_file_text
        .as_deref()
        .ok_or_else(|| "edit fixture is missing an lsp current_file block".to_string())?;
    let source_file_path = fixture.first_file_path()?;

    Ok((source_file_path, expected_text.to_string()))
}

/// Return the sorted file paths included in one workspace diagnostic report.
fn workspace_diagnostic_report_file_paths(
    workspace_root: &std::path::Path,
    report: &lsp::WorkspaceDiagnosticReportResult,
) -> Result<Vec<String>, String> {
    let lsp::WorkspaceDiagnosticReportResult::Report(report) = report else {
        return Err("expected full workspace diagnostic report".to_string());
    };
    let mut file_paths = Vec::new();

    // preserve exact document coverage independent of diagnostic contents
    for item in &report.items {
        let uri = match item {
            lsp::WorkspaceDocumentDiagnosticReport::Full(full) => &full.uri,
            lsp::WorkspaceDocumentDiagnosticReport::Unchanged(unchanged) => &unchanged.uri,
        };
        let path = uri
            .to_file_path()
            .ok_or_else(|| format!("workspace diagnostic uri is not a file path: {uri:?}"))?;
        let relative_path = path
            .strip_prefix(workspace_root)
            .map_err(|_| {
                format!(
                    "workspace diagnostic path {} is outside workspace root {}",
                    path.display(),
                    workspace_root.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");

        file_paths.push(relative_path);
    }

    file_paths.sort();

    Ok(file_paths)
}
