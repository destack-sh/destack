use crate::lsp::runner::primary_file_path;
use crate::lsp::{
    ExpectedCodeAction, FormatOptionValue, LspFixture, LspScenario, LspTestState,
    NormalizedCodeAction, normalize_code_actions, verify_code_actions, verify_exact_eq,
    verify_file_text, verify_workspace_edit_count, workspace_edit_edit_count,
};

const RENAME_TARGET_NAME: &str = "salute";

/// Run the refactor scenarios declared by one fixture.
pub(crate) fn run_refactor_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // rename
    if test_state.marker("rename").is_some() {
        test_state.go_to().marker("rename")?;

        let rename_edit = test_state
            .request_rename(RENAME_TARGET_NAME)?
            .ok_or_else(|| "expected rename workspace edit".to_string())?;
        let edit_count = workspace_edit_edit_count(&rename_edit);
        let expected_edit_count = test_state.fixture.ranges.len();

        verify_workspace_edit_count(edit_count, expected_edit_count)?;
    }

    // whole-document formatting
    if let Some(expected_text) = fixture
        .expectations
        .formatting
        .document_expected_text
        .as_deref()
    {
        let source_file_path = primary_file_path(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.format().document()?;

        test_state.verify().current_file_content_is(expected_text)?;
    }

    // whole-document formatting should also support exact no-op assertions
    if fixture.has_scenario(LspScenario::DocumentFormattingChangesNothing) {
        let source_file_path = fixture.first_file_path()?;

        test_state.go_to().file(&source_file_path)?;
        test_state.verify().format_document_changes_nothing()?;
    }

    // range formatting
    if !fixture.has_scenario(LspScenario::FormatSelectionMarkers)
        && let Some(expected_text) = fixture
            .expectations
            .formatting
            .range_expected_text
            .as_deref()
    {
        let source_file_path = primary_file_path(fixture)?;
        let range = fixture.ranges.first().cloned().ok_or_else(|| {
            "range formatting fixture is missing [|...|] selection range".to_string()
        })?;

        let edits = test_state
            .request_range_formatting_for_range(&range)?
            .ok_or_else(|| "expected range formatting edits".to_string())?;
        test_state.apply_text_edits(&source_file_path, &edits)?;
        let actual_text = test_state.current_document_text(&source_file_path)?;

        verify_file_text(actual_text, expected_text)?;
    }

    // selection formatting should flow through the tsserver-shaped format facade
    if fixture.has_scenario(LspScenario::FormatSelectionMarkers) {
        let expected_text = fixture
            .expectations
            .formatting
            .range_expected_text
            .as_deref()
            .ok_or_else(|| {
                "format selection fixture is missing an lsp range_formatting block".to_string()
            })?;
        let source_file_path = primary_file_path(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state
            .format()
            .selection("format_start", "format_end")?;

        test_state.verify().current_file_content_is(expected_text)?;
    }

    // on-type formatting should apply exact edits through the native format facade
    if fixture.has_scenario(LspScenario::OnTypeFormattingBrace) {
        let expected_text = fixture
            .expectations
            .formatting
            .current_file_text
            .as_deref()
            .ok_or_else(|| {
                "on-type formatting fixture is missing an lsp current_file block".to_string()
            })?;
        let source_file_path = primary_file_path(fixture)?;

        test_state.go_to().file(&source_file_path)?;
        test_state.format().on_type("on_type", "}")?;

        test_state.verify().current_file_content_is(expected_text)?;
    }

    // format options should roundtrip through the native tsserver-shaped facade
    if fixture.has_scenario(LspScenario::FormatOptionRoundtrip) {
        let source_file_path = fixture
            .files
            .iter()
            .map(|file| file.path.clone())
            .next()
            .ok_or_else(|| "format option fixture is missing a file".to_string())?;

        test_state.go_to().file(&source_file_path)?;
        let original_options = test_state.format().copy_format_options();

        verify_exact_eq("default tab size", &original_options.tab_size, &4)?;
        verify_exact_eq(
            "default insert spaces",
            &original_options.insert_spaces,
            &true,
        )?;

        test_state
            .format()
            .set_option("tabSize", FormatOptionValue::Number(2))?;
        test_state
            .format()
            .set_option("insertSpaces", FormatOptionValue::Bool(false))?;
        test_state
            .format()
            .set_option("trimFinalNewlines", FormatOptionValue::Bool(true))?;

        let updated_options = test_state.format().copy_format_options();
        verify_exact_eq("updated tab size", &updated_options.tab_size, &2)?;
        verify_exact_eq(
            "updated insert spaces",
            &updated_options.insert_spaces,
            &false,
        )?;
        verify_exact_eq(
            "updated trim final newlines",
            &updated_options.trim_final_newlines,
            &Some(true),
        )?;

        test_state
            .format()
            .set_format_options(original_options.clone());

        let restored_options = test_state.format().copy_format_options();
        verify_exact_eq(
            "restored format options",
            &restored_options,
            &original_options,
        )?;
    }

    // edit formatting toggles should gate application of format edits
    if fixture.has_scenario(LspScenario::FormatDisableEnableRoundtrip) {
        let expected_text = fixture
            .expectations
            .formatting
            .current_file_text
            .as_deref()
            .ok_or_else(|| {
                "format toggle fixture is missing an lsp current_file block".to_string()
            })?;
        let source_file_path = primary_file_path(fixture)?;
        let original_text = fixture
            .file(&source_file_path)
            .map(|file| file.text.clone())
            .ok_or_else(|| format!("format toggle fixture is missing file {source_file_path}"))?;

        test_state.go_to().file(&source_file_path)?;
        test_state.edit().disable_formatting();
        test_state.format().document()?;
        test_state
            .verify()
            .current_file_content_is(&original_text)?;

        test_state.edit().enable_formatting();
        test_state.format().document()?;
        test_state.verify().current_file_content_is(expected_text)?;
    }

    // code actions
    if !fixture.expectations.code_actions.actions.is_empty() {
        let source_file_path = fixture.first_file_path()?;
        let action_marker = fixture
            .marker("action")
            .ok_or_else(|| "code action fixture is missing /*action*/ marker".to_string())?;
        let expected_actions = fixture
            .expectations
            .code_actions
            .actions
            .iter()
            .map(normalize_expected_code_action)
            .collect::<Result<Vec<_>, _>>()?;

        test_state.go_to().marker(&action_marker.name)?;
        let actions = test_state
            .request_code_actions()?
            .ok_or_else(|| "expected code actions result".to_string())?;
        let actual_actions = normalize_code_actions(&actions)?;

        verify_code_actions(&actual_actions, &expected_actions)?;

        if let Some(expected_text) = fixture.expectations.code_actions.expected_text.as_deref() {
            let first_action = actions
                .into_iter()
                .next()
                .ok_or_else(|| "expected at least one code action".to_string())?;
            let destack_lsp_types::CodeActionOrCommand::CodeAction(first_action) = first_action
            else {
                return Err("expected first code action entry, not a bare command".to_string());
            };
            let expected_first_action = fixture
                .expectations
                .code_actions
                .actions
                .first()
                .ok_or_else(|| "expected at least one code action expectation".to_string())?;
            verify_code_action_edit_shape(expected_first_action, &first_action)?;
            let resolved_action = if first_action.edit.is_some() {
                first_action
            } else {
                test_state
                    .resolve_code_action(first_action)?
                    .ok_or_else(|| "expected resolved code action result".to_string())?
            };
            let edit = resolved_action
                .edit
                .ok_or_else(|| "expected first code action to yield workspace edits".to_string())?;
            let edits = edit
                .changes
                .as_ref()
                .and_then(|changes| {
                    let uri = test_state.driver.uri_for(&source_file_path);
                    changes.get(&uri)
                })
                .cloned()
                .ok_or_else(|| "expected first code action to edit the primary file".to_string())?;

            test_state.apply_text_edits(&source_file_path, &edits)?;
            let actual_text = test_state.current_document_text(&source_file_path)?;

            verify_file_text(actual_text, expected_text)?;
        }
    }

    Ok(())
}

/// Normalize one expected code action into the harness comparison shape.
fn normalize_expected_code_action(
    action: &ExpectedCodeAction,
) -> Result<NormalizedCodeAction, String> {
    Ok(NormalizedCodeAction {
        title: action.title.clone(),
        kind: normalize_expected_code_action_kind(&action.kind)?.to_string(),
        is_preferred: action.is_preferred,
        has_edit: action.has_edit,
    })
}

/// Normalize one expected code action kind into the harness label space.
fn normalize_expected_code_action_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "quick_fix" => Ok("quick_fix"),
        "refactor" => Ok("refactor"),
        "refactor_extract" => Ok("refactor_extract"),
        "refactor_inline" => Ok("refactor_inline"),
        "refactor_rewrite" => Ok("refactor_rewrite"),
        "source" => Ok("source"),
        "source_organize_imports" => Ok("source_organize_imports"),
        "source_fix_all" => Ok("source_fix_all"),
        _ => Err(format!("unsupported expected code action kind {kind}")),
    }
}

/// Verify whether one action should have eager or lazy edits before resolution.
fn verify_code_action_edit_shape(
    expected_action: &ExpectedCodeAction,
    actual_action: &destack_lsp_types::CodeAction,
) -> Result<(), String> {
    let actual_has_edit = actual_action.edit.is_some();

    if actual_has_edit == expected_action.has_edit {
        return Ok(());
    }

    Err(format!(
        "code action edit shape mismatch for {}\nactual: has_edit={actual_has_edit}\nexpected: has_edit={}",
        expected_action.title, expected_action.has_edit
    ))
}
