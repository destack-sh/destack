use std::collections::BTreeMap;
use std::path::Path;

use destack_lsp_server::jsonrpc::ErrorCode;
use destack_lsp_types as lsp;

use crate::core::CaseResult;
use crate::lsp::runner::{
    assists, commands, hierarchy, lifecycle, navigation, refactor, symbols, tokens,
};
use crate::lsp::{
    ExpectedDocumentSymbol, ExpectedWorkspaceSymbol, FormatOptionValue, LspFixture, LspStepCase,
    LspTestState, Marker, NormalizedCodeAction, NormalizedCompletionItem, NormalizedDiagnostic,
    NormalizedDocumentSymbol, NormalizedLocation, NormalizedWorkspaceSymbol, Range,
    ResolvedLspStepSnapshot, SignatureHelpTrigger, expected_symbol_kind_name,
    normalize_call_hierarchy_incoming, normalize_call_hierarchy_outgoing, normalize_code_actions,
    normalize_code_lenses, normalize_completion_response, normalize_definition_response,
    normalize_document_links, normalize_document_symbols, normalize_folding_ranges,
    normalize_inlay_hints, normalize_references_response, normalize_resolved_completion_item,
    normalize_semantic_tokens, normalize_type_hierarchy_items, normalize_workspace_symbols,
    parse_expected_call_hierarchy_calls, parse_expected_code_actions, parse_expected_code_lenses,
    parse_expected_completion_items, parse_expected_diagnostics_snapshot,
    parse_expected_document_links, parse_expected_document_symbols, parse_expected_folding_ranges,
    parse_expected_inlay_hints, parse_expected_resolved_completion, parse_expected_semantic_tokens,
    parse_expected_type_hierarchy_items, parse_expected_workspace_symbols,
    verify_call_hierarchy_calls, verify_code_actions, verify_code_lenses, verify_completion_items,
    verify_definition_locations, verify_diagnostics, verify_document_links,
    verify_document_symbols, verify_file_text, verify_folding_ranges, verify_inlay_hints,
    verify_reference_locations, verify_resolved_completion_item, verify_semantic_tokens,
    verify_type_hierarchy_items, verify_workspace_symbols,
};
use crate::mdtest::MdTestCase;

/// One cached semantic-token baseline captured before a step transition.
#[derive(Clone)]
struct SemanticTokenDeltaBaseline {
    /// The file path that owns this baseline.
    file_path: String,
    /// The semantic-token result captured before the transition.
    result: lsp::SemanticTokensResult,
}

/// Run one applied-LSP markdown test case.
pub(crate) fn run_mdtest_case(path: &Path, test: &MdTestCase) -> CaseResult {
    // parse the fixture before higher-level case execution begins
    let fixture = match LspFixture::from_mdtest(path, test) {
        Ok(fixture) => fixture,
        Err(error) => {
            return CaseResult::Failed {
                message: error.to_string(),
            };
        }
    };

    // bootstrap the editor-shaped test state against the materialized workspace
    let mut test_state = match LspTestState::from_fixture("applied_lsp", &fixture) {
        Ok(test_state) => test_state,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    // open the declared fixture files before semantic requests run
    if let Err(error) = test_state.open_fixture_files() {
        return CaseResult::Failed { message: error };
    }

    // explicit step sequences own their own execution order
    if fixture.has_step_sequence() {
        if fixture.has_declarative_cases() {
            return CaseResult::Failed {
                message: "stepped fixture still contains declarative runnable cases".to_string(),
            };
        }

        if let Err(error) = run_step_sequence(&fixture, &mut test_state) {
            return CaseResult::Failed { message: error };
        }

        return CaseResult::Passed;
    }

    // execute declared workspace commands before semantic request families run
    if let Err(error) = commands::run_command_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    // run the capability families in a stable order
    if let Err(error) = navigation::run_navigation_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = assists::run_assist_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = hierarchy::run_hierarchy_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = symbols::run_symbol_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = tokens::run_token_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = refactor::run_refactor_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    if let Err(error) = lifecycle::run_lifecycle_cases(&fixture, &mut test_state) {
        return CaseResult::Failed { message: error };
    }

    CaseResult::Passed
}

/// Run one explicit step-indexed fixture sequence.
fn run_step_sequence(fixture: &LspFixture, test_state: &mut LspTestState) -> Result<(), String> {
    let step_indices = fixture.step_indices();

    // step 0 assertions run against the initially opened workspace state
    if step_indices.first().copied() == Some(0) {
        let step_snapshot = resolved_step_snapshot(fixture, 0)?;
        let delta_baselines = BTreeMap::new();

        execute_step_cases(fixture, test_state, &step_snapshot, &delta_baselines)?;
    }

    // step 0 is already materialized through normal fixture bootstrap
    for window in step_indices.windows(2) {
        let current_step = window[0];
        let next_step = window[1];
        let current_snapshot = resolved_step_snapshot(fixture, current_step)?;
        let next_snapshot = resolved_step_snapshot(fixture, next_step)?;
        let delta_baselines =
            capture_delta_baselines(fixture, test_state, &current_snapshot, &next_snapshot)?;

        apply_step_transition(test_state, &current_snapshot, &next_snapshot)?;
        execute_step_cases(fixture, test_state, &next_snapshot, &delta_baselines)?;
    }

    Ok(())
}

/// Capture semantic-token baselines before one transition when the next step requests deltas.
fn capture_delta_baselines(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
    current_snapshot: &ResolvedLspStepSnapshot,
    next_snapshot: &ResolvedLspStepSnapshot,
) -> Result<BTreeMap<String, SemanticTokenDeltaBaseline>, String> {
    let mut baselines = BTreeMap::new();

    // capture only the files the next step will query through the delta path
    for step_case in fixture.step_cases(next_snapshot.index) {
        let LspStepCase::SemanticTokensDelta { file_path, .. } = step_case else {
            continue;
        };

        if baselines.contains_key(file_path) {
            continue;
        }

        let is_file_known = current_snapshot
            .files
            .iter()
            .any(|file| file.path == *file_path);
        if !is_file_known {
            return Err(format!(
                "semantic token delta step [{}] references unknown file {}",
                next_snapshot.index, file_path
            ));
        }

        test_state.go_to_file(file_path)?;

        let result = test_state
            .request_semantic_tokens_full()?
            .ok_or_else(|| "expected baseline semantic token result".to_string())?;
        let baseline_result_id = tokens::semantic_tokens_result_id(&result)?
            .ok_or_else(|| "expected baseline semantic token result id".to_string())?;

        // keep the baseline result id alive through the cached result object
        let _ = baseline_result_id;

        baselines.insert(
            file_path.clone(),
            SemanticTokenDeltaBaseline {
                file_path: file_path.clone(),
                result,
            },
        );
    }

    Ok(baselines)
}

/// Apply the workspace text transition between two declared steps.
fn apply_step_transition(
    test_state: &mut LspTestState,
    current_snapshot: &ResolvedLspStepSnapshot,
    next_snapshot: &ResolvedLspStepSnapshot,
) -> Result<(), String> {
    let current_files = current_snapshot
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    let next_files = next_snapshot
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut changed_file_count = 0usize;

    // create new files before later queries can see them
    for next_file in &next_snapshot.files {
        let Some(current_file) = current_files.get(next_file.path.as_str()) else {
            test_state.create_file_text(&next_file.path, &next_file.text)?;
            changed_file_count += 1;
            continue;
        };

        if current_file.text != next_file.text {
            // route open documents through didChange and closed files through watch events
            if test_state.is_file_open(&next_file.path) {
                test_state.replace_document_text(&next_file.path, &next_file.text)?;
            } else {
                test_state.replace_closed_file_text(&next_file.path, &next_file.text)?;
            }

            changed_file_count += 1;
        }
    }

    // delete files that disappear from the next workspace state
    for current_file in &current_snapshot.files {
        if next_files.contains_key(current_file.path.as_str()) {
            continue;
        }

        test_state.delete_file(&current_file.path)?;
        changed_file_count += 1;
    }

    // wait until the queued mutation lane drains before the next request wave starts
    if changed_file_count > 0 {
        test_state.wait_for_mutation_idle();
    }

    Ok(())
}

/// Execute all explicit step-owned cases for one step index.
fn execute_step_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
    step_snapshot: &ResolvedLspStepSnapshot,
    delta_baselines: &BTreeMap<String, SemanticTokenDeltaBaseline>,
) -> Result<(), String> {
    let step_index = step_snapshot.index;

    for step_case in fixture.step_cases(step_index) {
        match step_case {
            LspStepCase::OpenFile { file_path } => {
                test_state.open_file(file_path)?;
            }
            LspStepCase::GoToMarker { marker_name } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;
            }
            LspStepCase::SelectMarkers {
                start_marker_name,
                end_marker_name,
            } => {
                let start_marker =
                    marker_for_step(fixture, step_snapshot, step_index, start_marker_name)?;
                let end_marker =
                    marker_for_step(fixture, step_snapshot, step_index, end_marker_name)?;

                test_state.go_to_file(&start_marker.file_path)?;
                test_state.select_offsets_in_file(
                    &start_marker.file_path,
                    start_marker.offset,
                    end_marker.offset,
                )?;
            }
            LspStepCase::SelectAll { file_path } => {
                test_state.go_to_file(file_path)?;
                test_state.select_all_in_file(file_path)?;
            }
            LspStepCase::GoToBof => {
                test_state.go_to_bof()?;
            }
            LspStepCase::MoveRight { count } => {
                test_state.edit().move_right(*count)?;
            }
            LspStepCase::ReplaceSelection { text } => {
                test_state.edit().replace_selection(text)?;
            }
            LspStepCase::Paste { text } => {
                test_state.edit().paste(text)?;
            }
            LspStepCase::Backspace { count } => {
                test_state.edit().backspace(*count)?;
            }
            LspStepCase::DeleteAtCaret { count } => {
                test_state.edit().delete_at_caret(*count)?;
            }
            LspStepCase::DeleteLine { index } => {
                test_state.edit().delete_line(*index)?;
            }
            LspStepCase::DeleteLineRange {
                start_index,
                end_index_inclusive,
            } => {
                test_state
                    .edit()
                    .delete_line_range(*start_index, *end_index_inclusive)?;
            }
            LspStepCase::ReplaceLine { index, text } => {
                test_state.edit().replace_line(*index, text)?;
            }
            LspStepCase::OpenText { file_path, text } => {
                test_state.open_file_with_text(file_path, text)?;
            }
            LspStepCase::ExecuteCommand { command } => {
                test_state.execute_command(command)?;
            }
            LspStepCase::SaveFile { file_path } => {
                test_state.save_file(file_path)?;
            }
            LspStepCase::CloseFile { file_path } => {
                test_state.close_file(file_path)?;
            }
            LspStepCase::Definition {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let definition = test_state.request_definition()?.ok_or_else(|| {
                    format!("expected goto definition result at step {step_index}")
                })?;
                let actual_definition =
                    normalize_definition_response(test_state.workspace_root(), &definition)?;
                let expected_definition =
                    parse_expected_definition_snapshot(fixture, step_snapshot, snapshot_text)?;

                verify_definition_locations(&actual_definition, &expected_definition)?;
            }
            LspStepCase::References {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let references = test_state
                    .request_references(true)?
                    .ok_or_else(|| format!("expected references result at step {step_index}"))?;
                let actual_references =
                    normalize_references_response(test_state.workspace_root(), &references)?;
                let expected_references =
                    parse_expected_definition_snapshot(fixture, step_snapshot, snapshot_text)?;

                verify_reference_locations(&actual_references, &expected_references)?;
            }
            LspStepCase::QuickInfo { marker_name } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let signature = fixture
                    .expectations
                    .hover
                    .signature
                    .as_deref()
                    .ok_or_else(|| "quick info step is missing lsp hover_signature".to_string())?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;
                test_state.verify().quick_info_is(
                    signature,
                    fixture.expectations.hover.documentation.as_deref(),
                )?;
            }
            LspStepCase::Indentation {
                marker_name,
                number_of_spaces,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;
                test_state.verify().indentation_is(*number_of_spaces)?;
                test_state.verify().indentation_at_position_is(
                    &marker.file_path,
                    marker.offset,
                    *number_of_spaces,
                )?;
            }
            LspStepCase::NoSignatureHelp { marker_name } => {
                test_state
                    .verify()
                    .negated()
                    .no_signature_help(&[marker_name.as_str()])?;
            }
            LspStepCase::SignatureHelpTrigger {
                marker_name,
                trigger_character,
            } => {
                test_state
                    .verify()
                    .signature_help_present_for_trigger_reason(
                        &SignatureHelpTrigger::trigger_character(trigger_character),
                        &[marker_name.as_str()],
                    )?;
            }
            LspStepCase::NoSignatureHelpForTriggerReason { marker_name } => {
                test_state.verify().no_signature_help_for_trigger_reason(
                    &SignatureHelpTrigger::invoked(),
                    &[marker_name.as_str()],
                )?;
            }
            LspStepCase::DocumentDiagnostic {
                file_path,
                snapshot_text,
            } => {
                let report = test_state.request_document_diagnostics(file_path)?;
                let actual = normalize_document_diagnostics(file_path, report)?;
                let expected = parse_expected_diagnostics_snapshot(snapshot_text)?;

                verify_diagnostics(&actual, &expected)?;
            }
            LspStepCase::WorkspaceDiagnostic { snapshot_text } => {
                let report = test_state.request_workspace_diagnostics()?;
                let actual = crate::lsp::normalize_workspace_diagnostic_report(
                    test_state.workspace_root(),
                    &report,
                )?;
                let expected = parse_expected_diagnostics_snapshot(snapshot_text)?;

                verify_diagnostics(&actual, &expected)?;
            }
            LspStepCase::CurrentFile { file_path, text } => {
                let actual = test_state.current_document_text(file_path)?;

                verify_file_text(actual, text)?;
            }
            LspStepCase::CodeAction {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_actions = parse_expected_code_actions(snapshot_text)?
                    .into_iter()
                    .map(|action| normalize_expected_code_action_for_step(&action))
                    .collect::<Result<Vec<_>, _>>()?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let actions = test_state
                    .request_code_actions()?
                    .ok_or_else(|| format!("expected code actions result at step {step_index}"))?;
                let actual_actions = normalize_code_actions(&actions)?;

                verify_code_actions(&actual_actions, &expected_actions)?;
            }
            LspStepCase::Completion {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_items = parse_expected_completion_items(snapshot_text)?
                    .into_iter()
                    .map(|item| -> Result<NormalizedCompletionItem, String> {
                        Ok(NormalizedCompletionItem {
                            label: item.label,
                            kind: normalize_expected_completion_kind_for_step(&item.kind)?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let completion = test_state
                    .request_completion()?
                    .ok_or_else(|| format!("expected completion result at step {step_index}"))?;
                let actual_items = normalize_completion_response(&completion)?;

                verify_completion_items(&actual_items, &expected_items)?;
            }
            LspStepCase::CompletionResolve {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_resolved = parse_expected_resolved_completion(snapshot_text)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let completion = test_state
                    .request_completion()?
                    .ok_or_else(|| format!("expected completion result at step {step_index}"))?;
                let resolved_item = completion_item_by_label_for_step(
                    completion,
                    &expected_resolved.label,
                    step_index,
                )?
                .ok_or_else(|| {
                    format!(
                        "failed to find completion item {} at step {step_index}",
                        expected_resolved.label
                    )
                })?;
                let resolved_item =
                    test_state
                        .resolve_completion(resolved_item)?
                        .ok_or_else(|| {
                            format!("expected completion resolve result at step {step_index}")
                        })?;
                let actual_resolved = normalize_resolved_completion_item(&resolved_item)?;

                verify_resolved_completion_item(&actual_resolved, &expected_resolved)?;
            }
            LspStepCase::DocumentLink {
                file_path,
                snapshot_text,
            } => {
                let expected_links = parse_expected_document_links(snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let links = test_state.request_document_links()?.ok_or_else(|| {
                    format!("expected document links result at step {step_index}")
                })?;
                let actual_links = normalize_document_links(test_state.workspace_root(), &links)?;

                verify_document_links(&actual_links, &expected_links)?;
            }
            LspStepCase::DocumentSymbols {
                file_path,
                snapshot_text,
            } => {
                let expected_symbols =
                    expected_document_symbols_for_step(fixture, step_snapshot, snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let symbols = test_state.request_document_symbols()?.ok_or_else(|| {
                    format!("expected document symbols result at step {step_index}")
                })?;
                let actual_symbols =
                    normalize_document_symbols(test_state.workspace_root(), file_path, &symbols)?;

                verify_document_symbols(&actual_symbols, &expected_symbols)?;
            }
            LspStepCase::WorkspaceSymbols {
                query,
                snapshot_text,
            } => {
                let expected_symbols =
                    expected_workspace_symbols_for_step(fixture, step_snapshot, snapshot_text)?;
                let symbols = test_state.request_workspace_symbols(query)?;
                let actual_symbols = match symbols {
                    Some(response) => {
                        normalize_workspace_symbols(test_state.workspace_root(), &response)?
                    }
                    None => Vec::new(),
                };

                verify_workspace_symbols(&actual_symbols, &expected_symbols)?;
            }
            LspStepCase::FoldingRange {
                file_path,
                snapshot_text,
            } => {
                let expected_ranges = parse_expected_folding_ranges(snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let ranges = test_state.request_folding_ranges()?.ok_or_else(|| {
                    format!("expected folding ranges result at step {step_index}")
                })?;
                let actual_ranges = normalize_folding_ranges(&ranges);

                verify_folding_ranges(&actual_ranges, &expected_ranges)?;
            }
            LspStepCase::InlayHint {
                file_path,
                snapshot_text,
            } => {
                let expected_hints = parse_expected_inlay_hints(snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let hints = test_state
                    .request_inlay_hints()?
                    .ok_or_else(|| format!("expected inlay hints result at step {step_index}"))?;
                let actual_hints = normalize_inlay_hints(&hints)?;

                verify_inlay_hints(&actual_hints, &expected_hints)?;
            }
            LspStepCase::CodeLens {
                file_path,
                snapshot_text,
            } => {
                let expected_lenses = parse_expected_code_lenses(snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let lenses = test_state
                    .request_code_lenses()?
                    .ok_or_else(|| format!("expected code lenses result at step {step_index}"))?;
                let actual_lenses = normalize_code_lenses(&lenses);

                verify_code_lenses(&actual_lenses, &expected_lenses)?;

                if let Some(first_lens) = lenses.into_iter().next() {
                    let resolved_lens =
                        test_state.resolve_code_lens(first_lens)?.ok_or_else(|| {
                            format!("expected code lens resolve result at step {step_index}")
                        })?;
                    let actual_resolved = normalize_code_lenses(&[resolved_lens]);
                    let expected_resolved = expected_lenses.first().cloned().ok_or_else(|| {
                        format!("code lens step is missing expected lens at step {step_index}")
                    })?;

                    verify_code_lenses(&actual_resolved, &[expected_resolved])?;
                }
            }
            LspStepCase::SemanticTokens {
                file_path,
                snapshot_text,
            } => {
                let expected_tokens = parse_expected_semantic_tokens(snapshot_text)?;

                test_state.go_to_file(file_path)?;

                let result = test_state.request_semantic_tokens_full()?.ok_or_else(|| {
                    format!("expected semantic tokens result at step {step_index}")
                })?;
                let source_text = test_state.current_document_text(file_path)?;
                let actual_tokens = normalize_semantic_tokens(source_text, &result)?;

                verify_semantic_tokens(&actual_tokens, &expected_tokens)?;
            }
            LspStepCase::SemanticTokensDelta {
                file_path,
                snapshot_text,
            } => {
                let expected_tokens = parse_expected_semantic_tokens(snapshot_text)?;
                let baseline = delta_baselines.get(file_path).ok_or_else(|| {
                    format!(
                        "semantic token delta step [{step_index}] is missing cached baseline for {file_path}"
                    )
                })?;
                let baseline_result_id = tokens::semantic_tokens_result_id(&baseline.result)?
                    .ok_or_else(|| "expected baseline semantic token result id".to_string())?;

                test_state.go_to_file(file_path)?;

                let current_version = test_state.current_document_version(file_path)?;
                let _ = test_state.wait_for_diagnostics_version(file_path, current_version)?;

                let delta_result = test_state
                    .request_semantic_tokens_full_delta(baseline_result_id)?
                    .ok_or_else(|| {
                        format!("expected semantic token delta result at step {step_index}")
                    })?;
                let merged_result =
                    tokens::semantic_tokens_after_delta(&baseline.result, &delta_result)?;
                let source_text = test_state.current_document_text(&baseline.file_path)?;
                let actual_tokens = normalize_semantic_tokens(source_text, &merged_result)?;

                verify_semantic_tokens(&actual_tokens, &expected_tokens)?;
            }
            LspStepCase::CallHierarchyIncoming {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_calls = parse_expected_call_hierarchy_calls(snapshot_text)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let prepared_items =
                    test_state
                        .request_prepare_call_hierarchy()?
                        .ok_or_else(|| {
                            format!("expected call hierarchy prepare result at step {step_index}")
                        })?;
                let prepared_item = prepared_items.into_iter().next().ok_or_else(|| {
                    format!("expected one call hierarchy prepare item at step {step_index}")
                })?;
                let incoming_calls = test_state
                    .request_call_hierarchy_incoming(prepared_item)?
                    .unwrap_or_default();
                let actual_calls = normalize_call_hierarchy_incoming(
                    test_state.workspace_root(),
                    &incoming_calls,
                )?;

                verify_call_hierarchy_calls(&actual_calls, &expected_calls)?;
            }
            LspStepCase::CallHierarchyOutgoing {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_calls = parse_expected_call_hierarchy_calls(snapshot_text)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let prepared_items =
                    test_state
                        .request_prepare_call_hierarchy()?
                        .ok_or_else(|| {
                            format!("expected call hierarchy prepare result at step {step_index}")
                        })?;
                let prepared_item = prepared_items.into_iter().next().ok_or_else(|| {
                    format!("expected one call hierarchy prepare item at step {step_index}")
                })?;
                let outgoing_calls = test_state
                    .request_call_hierarchy_outgoing(prepared_item)?
                    .unwrap_or_default();
                let actual_calls = normalize_call_hierarchy_outgoing(
                    test_state.workspace_root(),
                    &outgoing_calls,
                )?;

                verify_call_hierarchy_calls(&actual_calls, &expected_calls)?;
            }
            LspStepCase::TypeHierarchySupertypes {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_items = parse_expected_type_hierarchy_items(snapshot_text)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let prepared_items =
                    test_state
                        .request_prepare_type_hierarchy()?
                        .ok_or_else(|| {
                            format!("expected type hierarchy prepare result at step {step_index}")
                        })?;
                let prepared_item = prepared_items.into_iter().next().ok_or_else(|| {
                    format!("expected one type hierarchy prepare item at step {step_index}")
                })?;
                let actual_items = test_state
                    .request_type_hierarchy_supertypes(prepared_item)?
                    .unwrap_or_default();
                let actual_items =
                    normalize_type_hierarchy_items(test_state.workspace_root(), &actual_items)?;

                verify_type_hierarchy_items(&actual_items, &expected_items)?;
            }
            LspStepCase::TypeHierarchySubtypes {
                marker_name,
                snapshot_text,
            } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;
                let expected_items = parse_expected_type_hierarchy_items(snapshot_text)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.go_to_offset(&marker.file_path, marker.offset)?;

                let prepared_items =
                    test_state
                        .request_prepare_type_hierarchy()?
                        .ok_or_else(|| {
                            format!("expected type hierarchy prepare result at step {step_index}")
                        })?;
                let prepared_item = prepared_items.into_iter().next().ok_or_else(|| {
                    format!("expected one type hierarchy prepare item at step {step_index}")
                })?;
                let actual_items = test_state
                    .request_type_hierarchy_subtypes(prepared_item)?
                    .unwrap_or_default();
                let actual_items =
                    normalize_type_hierarchy_items(test_state.workspace_root(), &actual_items)?;

                verify_type_hierarchy_items(&actual_items, &expected_items)?;
            }
            LspStepCase::NoErrors { file_path } => {
                test_state.go_to_file(file_path)?;
                test_state.verify().no_errors()?;
                test_state.verify().number_of_errors_in_current_file(0)?;
            }
            LspStepCase::FormatDocument { file_path } => {
                test_state.go_to_file(file_path)?;
                test_state.format().document()?;
            }
            LspStepCase::FormatSelection {
                start_marker_name,
                end_marker_name,
            } => {
                let start_marker =
                    marker_for_step(fixture, step_snapshot, step_index, start_marker_name)?;

                test_state.go_to_file(&start_marker.file_path)?;
                test_state
                    .format()
                    .selection(start_marker_name, end_marker_name)?;
            }
            LspStepCase::OnTypeFormatting { marker_name } => {
                let marker = marker_for_step(fixture, step_snapshot, step_index, marker_name)?;

                test_state.go_to_file(&marker.file_path)?;
                test_state.format().on_type(marker_name, "}")?;
            }
            LspStepCase::DisableFormatting => {
                test_state.edit().disable_formatting();
            }
            LspStepCase::EnableFormatting => {
                test_state.edit().enable_formatting();
            }
            LspStepCase::SetFormatOption { name, value } => {
                let format_value = parse_step_format_option_value(value);

                test_state.format().set_option(name, format_value)?;
            }
            LspStepCase::VerifyFormatOptions { snapshot_text } => {
                let expected_options = parse_expected_format_options(snapshot_text)?;
                let options = test_state.format().copy_format_options();

                crate::lsp::verify_exact_eq(
                    "format option tab size",
                    &options.tab_size,
                    &expected_options.tab_size,
                )?;
                crate::lsp::verify_exact_eq(
                    "format option insert spaces",
                    &options.insert_spaces,
                    &expected_options.insert_spaces,
                )?;
                crate::lsp::verify_exact_eq(
                    "format option trim final newlines",
                    &options.trim_final_newlines,
                    &expected_options.trim_final_newlines,
                )?;
            }
            LspStepCase::CancelReferencesRequest { marker_name } => {
                cancel_references_request(test_state, marker_name, false)?;
            }
            LspStepCase::CancelReferencesProgress { marker_name } => {
                cancel_references_request(test_state, marker_name, true)?;
            }
            LspStepCase::CancelReferencesRequestByPolicy { marker_name } => {
                cancel_references_by_policy(test_state, marker_name, false)?;
            }
            LspStepCase::CancelReferencesProgressByPolicy { marker_name } => {
                cancel_references_by_policy(test_state, marker_name, true)?;
            }
        }
    }

    Ok(())
}

/// Return one marker from the step snapshot when available, otherwise from the base fixture.
fn marker_for_step<'a>(
    _fixture: &'a LspFixture,
    step_snapshot: &'a ResolvedLspStepSnapshot,
    step_index: usize,
    marker_name: &str,
) -> Result<&'a Marker, String> {
    step_snapshot
        .markers
        .get(marker_name)
        .ok_or_else(|| format!("step #{step_index} is missing /*{marker_name}*/ marker"))
}

/// Parse one formatting option value from the compact step text.
fn parse_step_format_option_value(value_text: &str) -> FormatOptionValue {
    let trimmed = value_text.trim();

    // booleans
    if trimmed == "true" {
        return FormatOptionValue::Bool(true);
    }

    if trimmed == "false" {
        return FormatOptionValue::Bool(false);
    }

    // integers
    if let Ok(number) = trimmed.parse::<i32>() {
        return FormatOptionValue::Number(number);
    }

    FormatOptionValue::String(trimmed.to_string())
}

/// One parsed formatting-option expectation record.
struct ExpectedFormatOptions {
    /// The expected tab size.
    tab_size: u32,
    /// The expected insert-spaces flag.
    insert_spaces: bool,
    /// The expected trim-final-newlines flag.
    trim_final_newlines: Option<bool>,
}

/// Parse one exact formatting-option snapshot.
fn parse_expected_format_options(snapshot_text: &str) -> Result<ExpectedFormatOptions, String> {
    let mut tab_size = None;
    let mut insert_spaces = None;
    let mut trim_final_newlines = None;

    // parse one `key=value` pair per non-empty line
    for line in snapshot_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "expected format option line as key=value, got `{line}`"
            ));
        };

        match key.trim() {
            "tab_size" => {
                let value = value.trim().parse::<u32>().map_err(|error| {
                    format!("invalid tab_size format option `{value}`: {error}")
                })?;
                tab_size = Some(value);
            }
            "insert_spaces" => {
                let value = parse_bool_field("insert_spaces", value.trim())?;
                insert_spaces = Some(value);
            }
            "trim_final_newlines" => {
                let value = parse_bool_field("trim_final_newlines", value.trim())?;
                trim_final_newlines = Some(value);
            }
            other => {
                return Err(format!("unknown format option key `{other}`"));
            }
        }
    }

    let Some(tab_size) = tab_size else {
        return Err("format option snapshot is missing tab_size".to_string());
    };
    let Some(insert_spaces) = insert_spaces else {
        return Err("format option snapshot is missing insert_spaces".to_string());
    };

    Ok(ExpectedFormatOptions {
        tab_size,
        insert_spaces,
        trim_final_newlines,
    })
}

/// Parse one exact boolean field from one compact snapshot line.
fn parse_bool_field(label: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "expected {label} to be true or false, got `{value}`"
        )),
    }
}

/// Return one completion item with a unique matching label for step execution.
fn completion_item_by_label_for_step(
    completion: lsp::CompletionResponse,
    label: &str,
    step_index: usize,
) -> Result<Option<lsp::CompletionItem>, String> {
    let items = match completion {
        lsp::CompletionResponse::Array(items) => items,
        lsp::CompletionResponse::List(list) => list.items,
    };

    // require one unique match so step fixtures cannot bind to the wrong duplicate label
    let mut matches = items.into_iter().filter(|item| item.label == label);
    let first = matches.next();

    if matches.next().is_some() {
        return Err(format!(
            "completion step {step_index} matched multiple completion items with label {label}"
        ));
    }

    Ok(first)
}

/// Normalize one expected completion kind string into the harness label space.
fn normalize_expected_completion_kind_for_step(kind: &str) -> Result<&'static str, String> {
    match kind {
        "text" => Ok("text"),
        "method" => Ok("method"),
        "function" => Ok("function"),
        "constructor" => Ok("constructor"),
        "field" => Ok("field"),
        "variable" => Ok("variable"),
        "class" => Ok("class"),
        "interface" => Ok("interface"),
        "module" => Ok("module"),
        "property" => Ok("property"),
        "unit" => Ok("unit"),
        "value" => Ok("value"),
        "enum" => Ok("enum"),
        "keyword" => Ok("keyword"),
        "snippet" => Ok("snippet"),
        "color" => Ok("color"),
        "file" => Ok("file"),
        "reference" => Ok("reference"),
        "folder" => Ok("folder"),
        "enum_member" => Ok("enum_member"),
        "constant" => Ok("constant"),
        "struct" => Ok("struct"),
        "event" => Ok("event"),
        "operator" => Ok("operator"),
        "type_parameter" => Ok("type_parameter"),
        _ => Err(format!("unsupported expected completion kind {kind}")),
    }
}

/// Normalize one expected code action into the step comparison shape.
fn normalize_expected_code_action_for_step(
    action: &crate::lsp::ExpectedCodeAction,
) -> Result<NormalizedCodeAction, String> {
    Ok(NormalizedCodeAction {
        title: action.title.clone(),
        kind: normalize_expected_code_action_kind_for_step(&action.kind)?.to_string(),
        is_preferred: action.is_preferred,
        has_edit: action.has_edit,
    })
}

/// Normalize one expected code action kind into the harness label space.
fn normalize_expected_code_action_kind_for_step(kind: &str) -> Result<&'static str, String> {
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

/// Cancel one references request and assert that the server reports request cancellation.
fn cancel_references_request(
    test_state: &mut LspTestState,
    marker_name: &str,
    use_progress: bool,
) -> Result<(), String> {
    test_state.go_to().marker(marker_name)?;

    let request_id = if use_progress {
        let work_done_token = lsp::ProgressToken::String("refs-cancel".to_string());
        let request_id =
            test_state.start_references_with_progress(true, work_done_token.clone())?;

        test_state.wait_for_work_done_progress_kind(&work_done_token, "begin")?;
        test_state.cancellation().cancel_progress(work_done_token);
        request_id
    } else {
        let request_id = test_state.start_references_request(true)?;

        test_state.cancellation().cancel_request(request_id);
        request_id
    };

    let response = test_state.await_request(request_id)?;
    let error = response
        .error()
        .ok_or_else(|| "expected references cancellation error response".to_string())?;
    if error.code == ErrorCode::RequestCancelled {
        return Ok(());
    }

    Err(format!(
        "references cancellation returned unexpected error code {}\nerror: {error:#?}",
        error.code
    ))
}

/// Apply the auto-cancel policy to one references request and assert a cancelled response.
fn cancel_references_by_policy(
    test_state: &mut LspTestState,
    marker_name: &str,
    use_progress: bool,
) -> Result<(), String> {
    test_state.go_to().marker(marker_name)?;
    test_state.cancellation().set_cancelled(0);

    let request_id = if use_progress {
        let work_done_token = lsp::ProgressToken::String("refs-policy".to_string());

        test_state.start_references_with_progress(true, work_done_token)?
    } else {
        test_state.start_references_request(true)?
    };

    let response = test_state.await_request(request_id)?;
    test_state.cancellation().reset_cancelled();
    let error = response
        .error()
        .ok_or_else(|| "expected references cancellation error response".to_string())?;
    if error.code == ErrorCode::RequestCancelled {
        return Ok(());
    }

    Err(format!(
        "policy cancellation returned unexpected error code {}\nerror: {error:#?}",
        error.code
    ))
}

/// Normalize one document-diagnostic response into exact diagnostics.
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

    Ok(crate::lsp::normalize_diagnostics(
        file_path,
        &full.full_document_diagnostic_report.items,
    ))
}

/// Parse one exact definition snapshot into normalized locations.
fn parse_expected_definition_snapshot(
    fixture: &LspFixture,
    step_snapshot: &ResolvedLspStepSnapshot,
    snapshot_text: &str,
) -> Result<Vec<NormalizedLocation>, String> {
    let mut expected_locations = Vec::new();

    for line in snapshot_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(marker) = step_snapshot.markers.get(line) {
            expected_locations.push(expected_location_from_snapshot_marker(
                fixture,
                Some(step_snapshot),
                marker,
            )?);
            continue;
        }

        let (file_path, start_line, start_character, end_line, end_character) =
            parse_location_snapshot_line(line)?;
        expected_locations.push(NormalizedLocation {
            file_path,
            start_line,
            start_character,
            end_line,
            end_character,
        });
    }

    Ok(expected_locations)
}

/// Build exact expected document symbols from the active step snapshot text.
fn expected_document_symbols_for_step(
    fixture: &LspFixture,
    step_snapshot: &ResolvedLspStepSnapshot,
    snapshot_text: &str,
) -> Result<Vec<NormalizedDocumentSymbol>, String> {
    let expected_symbols = parse_expected_document_symbols(snapshot_text)?;
    let mut entries = expected_symbols
        .iter()
        .map(|symbol| expected_document_symbol_for_step(fixture, step_snapshot, symbol))
        .collect::<Result<Vec<_>, _>>()?;

    // derive hierarchy depth from the smallest containing symbol range
    for index in 0..entries.len() {
        let depth = entries
            .iter()
            .enumerate()
            .filter(|(parent_index, parent)| {
                *parent_index != index && parent.range.contains(&entries[index].range)
            })
            .count();
        entries[index].depth = depth;
    }

    // compare document symbols in source order
    entries.sort_by_key(|symbol| {
        (
            symbol.range.file_path.clone(),
            symbol.range.start_line,
            symbol.range.start_character,
            symbol.depth,
        )
    });

    Ok(entries)
}

/// Build one expected normalized document symbol from the active step snapshot.
fn expected_document_symbol_for_step(
    fixture: &LspFixture,
    step_snapshot: &ResolvedLspStepSnapshot,
    symbol: &ExpectedDocumentSymbol,
) -> Result<NormalizedDocumentSymbol, String> {
    let marker = marker_for_step(fixture, step_snapshot, 0, &symbol.marker_name)?;
    let range = range_for_step_marker(fixture, Some(step_snapshot), marker).ok_or_else(|| {
        format!(
            "document symbol step is missing [|...|] range around /*{}*/",
            symbol.marker_name
        )
    })?;
    let selection_range =
        expected_location_from_snapshot_marker(fixture, Some(step_snapshot), marker)?;

    Ok(NormalizedDocumentSymbol {
        depth: 0,
        name: symbol_name_for_step_marker(fixture, Some(step_snapshot), marker)?,
        kind: expected_symbol_kind_name(&symbol.kind)?,
        range: NormalizedLocation {
            file_path: range.file_path.clone(),
            start_line: range.start_line,
            start_character: range.start_character,
            end_line: range.end_line,
            end_character: range.end_character,
        },
        selection_range,
    })
}

/// Build exact expected workspace symbols from the active step snapshot text.
fn expected_workspace_symbols_for_step(
    fixture: &LspFixture,
    step_snapshot: &ResolvedLspStepSnapshot,
    snapshot_text: &str,
) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
    let expected_symbols = parse_expected_workspace_symbols(snapshot_text)?;

    expected_symbols
        .iter()
        .map(|symbol| expected_workspace_symbol_for_step(fixture, step_snapshot, symbol))
        .collect()
}

/// Build one expected normalized workspace symbol from the active step snapshot.
fn expected_workspace_symbol_for_step(
    fixture: &LspFixture,
    step_snapshot: &ResolvedLspStepSnapshot,
    symbol: &ExpectedWorkspaceSymbol,
) -> Result<NormalizedWorkspaceSymbol, String> {
    let marker = marker_for_step(fixture, step_snapshot, 0, &symbol.marker_name)?;
    let range = range_for_step_marker(fixture, Some(step_snapshot), marker).ok_or_else(|| {
        format!(
            "workspace symbol step is missing [|...|] range around /*{}*/",
            symbol.marker_name
        )
    })?;

    Ok(NormalizedWorkspaceSymbol {
        name: symbol_name_for_step_marker(fixture, Some(step_snapshot), marker)?,
        kind: expected_symbol_kind_name(&symbol.kind)?,
        location: NormalizedLocation {
            file_path: range.file_path.clone(),
            start_line: range.start_line,
            start_character: range.start_character,
            end_line: range.end_line,
            end_character: range.end_character,
        },
        container_name: symbol.container_name.clone(),
    })
}

/// Build one expected normalized location from a marker in the current step snapshot.
fn expected_location_from_snapshot_marker(
    fixture: &LspFixture,
    step_snapshot: Option<&ResolvedLspStepSnapshot>,
    marker: &Marker,
) -> Result<NormalizedLocation, String> {
    let definition_width = symbol_width_at_step_marker(fixture, step_snapshot, marker)
        .ok_or_else(|| "failed to derive symbol width at definition marker".to_string())?;

    Ok(NormalizedLocation {
        file_path: marker.file_path.clone(),
        start_line: marker.line,
        start_character: marker.character,
        end_line: marker.line,
        end_character: marker.character + definition_width,
    })
}

/// Parse one compact location snapshot line.
fn parse_location_snapshot_line(
    line: &str,
) -> Result<(String, usize, usize, usize, usize), String> {
    let Some((file_path, range)) = line.split_once(':') else {
        return Err(format!("invalid definition snapshot line {line:?}"));
    };
    let Some((start, end)) = range.split_once('-') else {
        return Err(format!("invalid definition snapshot range {line:?}"));
    };
    let (start_line, start_character) = parse_line_character(start)?;
    let (end_line, end_character) = parse_line_character(end)?;

    Ok((
        file_path.to_string(),
        start_line,
        start_character,
        end_line,
        end_character,
    ))
}

/// Parse one `line:character` coordinate pair.
fn parse_line_character(value: &str) -> Result<(usize, usize), String> {
    let Some((line, character)) = value.split_once(':') else {
        return Err(format!("invalid line:character pair {value:?}"));
    };
    let line = line
        .parse::<usize>()
        .map_err(|error| format!("invalid line index {line:?}: {error}"))?;
    let character = character
        .parse::<usize>()
        .map_err(|error| format!("invalid character index {character:?}: {error}"))?;

    Ok((line, character))
}

/// Build one expected normalized location from a definition marker.
pub(crate) fn expected_location_from_marker(
    fixture: &LspFixture,
    marker: &Marker,
) -> Result<NormalizedLocation, String> {
    let definition_width = symbol_width_at_marker(fixture, marker)
        .ok_or_else(|| "failed to derive symbol width at definition marker".to_string())?;

    Ok(NormalizedLocation {
        file_path: marker.file_path.clone(),
        start_line: marker.line,
        start_character: marker.character,
        end_line: marker.line,
        end_character: marker.character + definition_width,
    })
}

/// Return the parsed range that contains the marker position.
pub(crate) fn range_for_marker<'a>(fixture: &'a LspFixture, marker: &Marker) -> Option<&'a Range> {
    fixture
        .ranges
        .iter()
        .filter(|range| {
            range.file_path == marker.file_path
                && range.start_offset <= marker.offset
                && marker.offset <= range.end_offset
        })
        .min_by_key(|range| range.end_offset - range.start_offset)
}

/// Return the parsed step range that contains the marker position.
fn range_for_step_marker<'a>(
    fixture: &'a LspFixture,
    step_snapshot: Option<&'a ResolvedLspStepSnapshot>,
    marker: &Marker,
) -> Option<&'a Range> {
    step_snapshot
        .and_then(|snapshot| {
            snapshot
                .ranges
                .iter()
                .filter(|range| {
                    range.file_path == marker.file_path
                        && range.start_offset <= marker.offset
                        && marker.offset <= range.end_offset
                })
                .min_by_key(|range| range.end_offset - range.start_offset)
        })
        .or_else(|| range_for_marker(fixture, marker))
}

/// Return one fully resolved step snapshot or fail loudly.
fn resolved_step_snapshot(
    fixture: &LspFixture,
    step_index: usize,
) -> Result<ResolvedLspStepSnapshot, String> {
    fixture
        .resolved_step_snapshot(step_index)
        .ok_or_else(|| format!("fixture is missing resolved step snapshot [{step_index}]"))
}

/// Return the primary file path for one fixture.
pub(crate) fn primary_file_path(fixture: &LspFixture) -> Result<String, String> {
    fixture.first_file_path()
}

/// Measure the identifier-like symbol width at one marker position.
fn symbol_width_at_marker(fixture: &LspFixture, marker: &Marker) -> Option<usize> {
    let file = fixture.file(&marker.file_path)?;
    let tail = file.text.get(marker.offset..)?;
    let width = tail
        .chars()
        .take_while(|character: &char| character.is_ascii_alphanumeric() || *character == '_')
        .count();

    (width > 0).then_some(width)
}

/// Measure the identifier-like symbol width at one marker for the active step.
fn symbol_width_at_step_marker(
    fixture: &LspFixture,
    step_snapshot: Option<&ResolvedLspStepSnapshot>,
    marker: &Marker,
) -> Option<usize> {
    let file = step_snapshot
        .and_then(|snapshot| {
            snapshot
                .files
                .iter()
                .find(|file| file.path == marker.file_path)
        })
        .or_else(|| fixture.file(&marker.file_path))?;
    let tail = file.text.get(marker.offset..)?;
    let width = tail
        .chars()
        .take_while(|character: &char| character.is_ascii_alphanumeric() || *character == '_')
        .count();

    (width > 0).then_some(width)
}

/// Read one identifier-like symbol name starting at the step marker offset.
fn symbol_name_for_step_marker(
    fixture: &LspFixture,
    step_snapshot: Option<&ResolvedLspStepSnapshot>,
    marker: &Marker,
) -> Result<String, String> {
    let file = step_snapshot
        .and_then(|snapshot| {
            snapshot
                .files
                .iter()
                .find(|file| file.path == marker.file_path)
        })
        .or_else(|| fixture.file(&marker.file_path))
        .ok_or_else(|| format!("fixture is missing file {}", marker.file_path))?;
    let tail = file
        .text
        .get(marker.offset..)
        .ok_or_else(|| format!("marker {} does not align to a char boundary", marker.name))?;
    let width = tail
        .chars()
        .take_while(|character: &char| character.is_ascii_alphanumeric() || *character == '_')
        .count();
    if width == 0 {
        return Err(format!(
            "failed to derive symbol width at marker {}",
            marker.name
        ));
    }

    Ok(tail.chars().take(width).collect())
}
