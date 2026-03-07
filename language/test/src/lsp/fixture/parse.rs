use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::lsp::fixture::{
    FixtureParseError, LspExpectations, LspFixture, LspSourceFile, LspStepCase, LspStepSnapshot,
    Marker, Range, parse_expected_code_actions, parse_expected_completion_items,
    parse_expected_document_symbols, parse_expected_workspace_symbols,
};
use crate::mdtest::{MdTestCase, MdTestFile, RawCodeBlock};

const LSP_BLOCK_PREFIX: &str = "lsp";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RangeOffset {
    /// The file path that owns this parsed range.
    file_path: String,
    /// The stripped byte start offset.
    start_offset: usize,
    /// The stripped byte end offset.
    end_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedFileText {
    /// The stripped output text.
    text: String,
    /// The markers declared in this file.
    markers: Vec<Marker>,
    /// The ranges declared in this file.
    ranges: Vec<RangeOffset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureMetadata {
    /// The grouped expectations declared by fixture metadata.
    expectations: LspExpectations,
    /// The explicit step-owned actions and assertions.
    step_cases: BTreeMap<usize, Vec<LspStepCase>>,
    /// The files removed at each step before later assertions run.
    step_removed_file_paths: BTreeMap<usize, Vec<String>>,
    /// The declarative runnable block kinds seen in this fixture.
    declarative_case_kinds: BTreeSet<String>,
    /// Whether the code-action resolve support flag has been set.
    code_action_resolve_support_is_set: bool,
}

impl FixtureMetadata {
    /// Build one empty metadata accumulator.
    fn new() -> Self {
        Self {
            expectations: LspExpectations::default(),
            step_cases: BTreeMap::new(),
            step_removed_file_paths: BTreeMap::new(),
            declarative_case_kinds: BTreeSet::new(),
            code_action_resolve_support_is_set: false,
        }
    }
}

/// Parse one markdown mdtest case into the native applied-LSP harness model.
pub(super) fn parse_mdtest_case(
    path: &Path,
    test: &MdTestCase,
) -> Result<LspFixture, FixtureParseError> {
    let mut metadata = FixtureMetadata::new();

    // parse all explicit lsp expectation blocks first
    for block in &test.extra_blocks {
        parse_metadata_block(path, test.line, block, &mut metadata)?;
    }

    let mut parsed_files = Vec::<LspSourceFile>::new();
    let mut step_snapshots = BTreeMap::<usize, LspStepSnapshot>::new();
    let mut markers = BTreeMap::<String, Marker>::new();
    let mut ranges = Vec::<Range>::new();

    // parse fixture files with marker stripping and keep source text exact
    for file in &test.files {
        let (file_path, step_index) = parse_versioned_file_path(path, test.line, &file.path)?;
        let parsed_file = parse_file_text(
            path,
            &MdTestFile {
                path: file_path.clone(),
                content: file.content.clone(),
                options: file.options.clone(),
            },
        )?;
        let line_starts = compute_line_starts(&parsed_file.text);
        let step_snapshot = step_snapshots
            .entry(step_index)
            .or_insert_with(|| LspStepSnapshot {
                index: step_index,
                files: Vec::new(),
                removed_file_paths: Vec::new(),
                markers: BTreeMap::new(),
                ranges: Vec::new(),
            });

        if step_snapshot
            .files
            .iter()
            .any(|existing| existing.path == file_path)
        {
            return Err(parse_error(
                path,
                test.line,
                &format!("duplicate source block for {file_path} at step {step_index}"),
            ));
        }

        for marker in parsed_file.markers {
            if step_index == 0 && markers.contains_key(&marker.name) {
                return Err(FixtureParseError {
                    path: path.to_path_buf(),
                    line: test.line,
                    message: format!("duplicate marker name {}", marker.name),
                });
            }

            let (line, character) = offset_to_line_character(&line_starts, marker.offset);
            let marker = Marker {
                line,
                character,
                ..marker
            };

            if step_snapshot.markers.contains_key(&marker.name) {
                return Err(parse_error(
                    path,
                    test.line,
                    &format!(
                        "duplicate marker name {} at step {}",
                        marker.name, step_index
                    ),
                ));
            }

            step_snapshot
                .markers
                .insert(marker.name.clone(), marker.clone());

            if step_index == 0 {
                markers.insert(marker.name.clone(), marker);
            }
        }

        for range in parsed_file.ranges {
            let (start_line, start_character) =
                offset_to_line_character(&line_starts, range.start_offset);
            let (end_line, end_character) =
                offset_to_line_character(&line_starts, range.end_offset);
            let text = parsed_file.text[range.start_offset..range.end_offset].to_string();

            let range = Range {
                file_path: range.file_path,
                start_offset: range.start_offset,
                end_offset: range.end_offset,
                start_line,
                start_character,
                end_line,
                end_character,
                text,
            };

            step_snapshot.ranges.push(range.clone());

            if step_index == 0 {
                ranges.push(range);
            }
        }

        let source_file = LspSourceFile {
            path: file_path,
            text: parsed_file.text,
        };

        step_snapshot.files.push(source_file.clone());

        if step_index == 0 {
            parsed_files.push(source_file);
        }
    }

    // merge step-owned file removals into the resolved snapshots
    for (step_index, removed_file_paths) in metadata.step_removed_file_paths.iter() {
        let step_snapshot = step_snapshots
            .entry(*step_index)
            .or_insert_with(|| LspStepSnapshot {
                index: *step_index,
                files: Vec::new(),
                removed_file_paths: Vec::new(),
                markers: BTreeMap::new(),
                ranges: Vec::new(),
            });

        step_snapshot
            .removed_file_paths
            .extend(removed_file_paths.iter().cloned());
    }

    // require at least one file so empty fixtures fail loudly
    if parsed_files.is_empty() {
        return Err(parse_error(
            path,
            test.line,
            "fixture did not declare any files",
        ));
    }

    let fixture = LspFixture {
        path: path.to_path_buf(),
        files: parsed_files,
        step_snapshots,
        markers,
        ranges,
        step_cases: metadata.step_cases,
        expectations: metadata.expectations,
    };

    fixture
        .validate_step_sequence()
        .map_err(|message| parse_error(path, test.line, &message))?;

    // reject mixed declarative and stepped runnable cases loudly
    if fixture.has_step_sequence() && !metadata.declarative_case_kinds.is_empty() {
        let case_kinds = metadata
            .declarative_case_kinds
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");

        return Err(parse_error(
            path,
            test.line,
            &format!(
                "stepped fixtures cannot mix explicit [n] cases with declarative runnable blocks: {case_kinds}"
            ),
        ));
    }

    Ok(fixture)
}

/// Parse one `lsp ...` block and update the grouped fixture metadata.
fn parse_metadata_block(
    path: &Path,
    line: usize,
    block: &RawCodeBlock,
    metadata: &mut FixtureMetadata,
) -> Result<(), FixtureParseError> {
    let mut parts = block.language.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() || parts[0] != LSP_BLOCK_PREFIX {
        return Ok(());
    }

    // reject malformed empty lsp blocks loudly
    if parts.len() < 2 {
        return Err(parse_error(path, line, "expected `lsp <kind> ...` block"));
    }

    let step_index = take_step_suffix(&mut parts, path, line)?;
    let kind = parts[1];
    let raw_body = block.content.as_str();
    let body = raw_body.trim_end();

    if let Some(step_index) = step_index
        && parse_step_block(path, line, &parts, raw_body, body, step_index, metadata)?
    {
        return Ok(());
    }

    // singleton scalar expectations
    if parse_scalar_block(path, line, kind, body, metadata)? {
        return Ok(());
    }

    // singleton snapshot expectations
    if parse_snapshot_block_kind(path, line, kind, raw_body, metadata)? {
        record_declarative_case_kind(kind, metadata);
        return Ok(());
    }

    // source overlays and feature flags
    if parse_source_block_kind(path, line, kind, raw_body, body, &parts, metadata)? {
        if is_declarative_source_case_kind(kind) {
            record_declarative_case_kind(kind, metadata);
        }

        return Ok(());
    }

    // repeated line-oriented expectations
    if parse_repeated_block(path, line, kind, body, metadata)? {
        record_declarative_case_kind(kind, metadata);
        return Ok(());
    }

    Err(parse_error(
        path,
        line,
        &format!("unknown lsp block kind {kind}"),
    ))
}

/// Parse one stepped `lsp ...` block when the block declares a step suffix.
fn parse_step_block(
    path: &Path,
    line: usize,
    parts: &[&str],
    raw_body: &str,
    body: &str,
    step_index: usize,
    metadata: &mut FixtureMetadata,
) -> Result<bool, FixtureParseError> {
    let kind = parts[1];

    match kind {
        "open" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(path, line, "expected `lsp open <path> [step]`"));
            };

            if !body.is_empty() {
                return Err(parse_error(
                    path,
                    line,
                    "lsp open step blocks must be empty",
                ));
            }

            push_step_case(
                metadata,
                step_index,
                LspStepCase::OpenFile {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "go_to_marker" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::GoToMarker { marker_name }
            })?
        }
        "select_markers" => {
            let Some(start_marker_name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp select_markers <start> <end> [step]`",
                ));
            };
            let Some(end_marker_name) = parts.get(3) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp select_markers <start> <end> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::SelectMarkers {
                    start_marker_name: (*start_marker_name).to_string(),
                    end_marker_name: (*end_marker_name).to_string(),
                },
            );
        }
        "select_all" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp select_all <path> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::SelectAll {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "go_to_bof" => push_step_case(metadata, step_index, LspStepCase::GoToBof),
        "move_right" => {
            let count = parse_usize_step_argument(path, line, parts.get(2).copied(), "move_right")?;

            push_step_case(metadata, step_index, LspStepCase::MoveRight { count });
        }
        "replace_selection" => push_step_case(
            metadata,
            step_index,
            LspStepCase::ReplaceSelection {
                text: normalize_step_text(raw_body),
            },
        ),
        "paste" => push_step_case(
            metadata,
            step_index,
            LspStepCase::Paste {
                text: normalize_step_text(raw_body),
            },
        ),
        "backspace" => {
            let count = parse_usize_step_argument(path, line, parts.get(2).copied(), "backspace")?;

            push_step_case(metadata, step_index, LspStepCase::Backspace { count });
        }
        "delete_at_caret" => {
            let count =
                parse_usize_step_argument(path, line, parts.get(2).copied(), "delete_at_caret")?;

            push_step_case(metadata, step_index, LspStepCase::DeleteAtCaret { count });
        }
        "delete_line" => {
            let index =
                parse_usize_step_argument(path, line, parts.get(2).copied(), "delete_line")?;

            push_step_case(metadata, step_index, LspStepCase::DeleteLine { index });
        }
        "delete_line_range" => {
            let start_index =
                parse_usize_step_argument(path, line, parts.get(2).copied(), "delete_line_range")?;
            let end_index_inclusive =
                parse_usize_step_argument(path, line, parts.get(3).copied(), "delete_line_range")?;

            push_step_case(
                metadata,
                step_index,
                LspStepCase::DeleteLineRange {
                    start_index,
                    end_index_inclusive,
                },
            );
        }
        "replace_line" => {
            let index =
                parse_usize_step_argument(path, line, parts.get(2).copied(), "replace_line")?;

            push_step_case(
                metadata,
                step_index,
                LspStepCase::ReplaceLine {
                    index,
                    text: normalize_step_text(raw_body),
                },
            );
        }
        "execute_command" => {
            let Some(command) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp execute_command <command> [step]`",
                ));
            };

            if !body.is_empty() {
                return Err(parse_error(
                    path,
                    line,
                    "lsp execute_command step blocks must be empty",
                ));
            }

            push_step_case(
                metadata,
                step_index,
                LspStepCase::ExecuteCommand {
                    command: (*command).to_string(),
                },
            );
        }
        "open_text" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp open_text <path> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::OpenText {
                    file_path: (*file_path).to_string(),
                    text: normalize_source_text(path, file_path, raw_body)?,
                },
            );
        }
        "delete_file" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp delete_file <path> [step]`",
                ));
            };

            if !body.is_empty() {
                return Err(parse_error(
                    path,
                    line,
                    "lsp delete_file step blocks must be empty",
                ));
            }

            metadata
                .step_removed_file_paths
                .entry(step_index)
                .or_default()
                .push((*file_path).to_string());
        }
        "save" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(path, line, "expected `lsp save <path> [step]`"));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::SaveFile {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "close" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp close <path> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::CloseFile {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "definition" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::Definition {
                marker_name,
                snapshot_text,
            },
        )?,
        "references" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::References {
                marker_name,
                snapshot_text,
            },
        )?,
        "quick_info_exists" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::QuickInfoExists { marker_name }
            })?
        }
        "quick_info" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::QuickInfo { marker_name }
            })?
        }
        "indentation" => {
            let Some(marker_name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp indentation <marker> [step]`",
                ));
            };
            let number_of_spaces = body.parse::<usize>().map_err(|error| {
                parse_error(path, line, &format!("invalid indentation width: {error}"))
            })?;

            push_step_case(
                metadata,
                step_index,
                LspStepCase::Indentation {
                    marker_name: (*marker_name).to_string(),
                    number_of_spaces,
                },
            );
        }
        "no_signature_help" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::NoSignatureHelp { marker_name }
            })?
        }
        "signature_help_trigger" => {
            let Some(marker_name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp signature_help_trigger <marker> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::SignatureHelpTrigger {
                    marker_name: (*marker_name).to_string(),
                    trigger_character: body.to_string(),
                },
            );
        }
        "no_signature_help_for_trigger_reason" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::NoSignatureHelpForTriggerReason { marker_name }
            })?
        }
        "document_diagnostic" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::DocumentDiagnostic {
                file_path,
                snapshot_text,
            },
        )?,
        "workspace_diagnostic" => {
            push_step_case(
                metadata,
                step_index,
                LspStepCase::WorkspaceDiagnostic {
                    snapshot_text: normalize_snapshot_text(raw_body),
                },
            );
        }
        "current_file" => push_file_text_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, text| LspStepCase::CurrentFile { file_path, text },
        )?,
        "code_action" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::CodeAction {
                marker_name,
                snapshot_text,
            },
        )?,
        "completion" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::Completion {
                marker_name,
                snapshot_text,
            },
        )?,
        "completion_resolve" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::CompletionResolve {
                marker_name,
                snapshot_text,
            },
        )?,
        "document_link" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::DocumentLink {
                file_path,
                snapshot_text,
            },
        )?,
        "document_symbols" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::DocumentSymbols {
                file_path,
                snapshot_text,
            },
        )?,
        "workspace_symbols" => push_query_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |query, snapshot_text| LspStepCase::WorkspaceSymbols {
                query,
                snapshot_text,
            },
        )?,
        "folding_range" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::FoldingRange {
                file_path,
                snapshot_text,
            },
        )?,
        "inlay_hint" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::InlayHint {
                file_path,
                snapshot_text,
            },
        )?,
        "code_lens" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::CodeLens {
                file_path,
                snapshot_text,
            },
        )?,
        "semantic_tokens" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::SemanticTokens {
                file_path,
                snapshot_text,
            },
        )?,
        "semantic_tokens_delta" => push_file_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |file_path, snapshot_text| LspStepCase::SemanticTokensDelta {
                file_path,
                snapshot_text,
            },
        )?,
        "call_hierarchy_incoming" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::CallHierarchyIncoming {
                marker_name,
                snapshot_text,
            },
        )?,
        "call_hierarchy_outgoing" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::CallHierarchyOutgoing {
                marker_name,
                snapshot_text,
            },
        )?,
        "type_hierarchy_supertypes" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::TypeHierarchySupertypes {
                marker_name,
                snapshot_text,
            },
        )?,
        "type_hierarchy_subtypes" => push_marker_snapshot_step_case(
            path,
            line,
            parts,
            step_index,
            metadata,
            raw_body,
            |marker_name, snapshot_text| LspStepCase::TypeHierarchySubtypes {
                marker_name,
                snapshot_text,
            },
        )?,
        "diagnostic_marker_windows" => {
            let Some(start_marker_name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp diagnostic_marker_windows <start> <end> [step]`",
                ));
            };
            let Some(end_marker_name) = parts.get(3) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp diagnostic_marker_windows <start> <end> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::DiagnosticMarkerWindows {
                    start_marker_name: (*start_marker_name).to_string(),
                    end_marker_name: (*end_marker_name).to_string(),
                },
            );
        }
        "no_errors" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp no_errors <path> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::NoErrors {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "format_document" => {
            let Some(file_path) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp format_document <path> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::FormatDocument {
                    file_path: (*file_path).to_string(),
                },
            );
        }
        "format_selection" => {
            let Some(start_marker_name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp format_selection <start> <end> [step]`",
                ));
            };
            let Some(end_marker_name) = parts.get(3) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp format_selection <start> <end> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::FormatSelection {
                    start_marker_name: (*start_marker_name).to_string(),
                    end_marker_name: (*end_marker_name).to_string(),
                },
            );
        }
        "on_type_formatting" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::OnTypeFormatting { marker_name }
            })?
        }
        "disable_formatting" => {
            push_step_case(metadata, step_index, LspStepCase::DisableFormatting)
        }
        "enable_formatting" => push_step_case(metadata, step_index, LspStepCase::EnableFormatting),
        "set_format_option" => {
            let Some(name) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp set_format_option <name> [step]`",
                ));
            };

            push_step_case(
                metadata,
                step_index,
                LspStepCase::SetFormatOption {
                    name: (*name).to_string(),
                    value: body.to_string(),
                },
            );
        }
        "verify_format_options" => push_step_case(
            metadata,
            step_index,
            LspStepCase::VerifyFormatOptions {
                snapshot_text: normalize_snapshot_text(raw_body),
            },
        ),
        "cancel_references_request" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::CancelReferencesRequest { marker_name }
            })?
        }
        "cancel_references_progress" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::CancelReferencesProgress { marker_name }
            })?
        }
        "cancel_references_request_by_policy" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::CancelReferencesRequestByPolicy { marker_name }
            })?
        }
        "cancel_references_progress_by_policy" => {
            push_marker_empty_step_case(path, line, parts, step_index, metadata, |marker_name| {
                LspStepCase::CancelReferencesProgressByPolicy { marker_name }
            })?
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Record one declarative runnable block kind for mixed-mode validation.
fn record_declarative_case_kind(kind: &str, metadata: &mut FixtureMetadata) {
    metadata.declarative_case_kinds.insert(kind.to_string());
}

/// Return whether one source-style block kind is a declarative runnable case.
fn is_declarative_source_case_kind(kind: &str) -> bool {
    matches!(kind, "code_action_result" | "semantic_tokens_delta_source")
}

/// Parse one required usize step argument.
fn parse_usize_step_argument(
    path: &Path,
    line: usize,
    argument: Option<&str>,
    kind: &str,
) -> Result<usize, FixtureParseError> {
    let Some(argument) = argument else {
        return Err(parse_error(
            path,
            line,
            &format!("expected `lsp {kind} ... [step]`"),
        ));
    };

    argument.parse::<usize>().map_err(|error| {
        parse_error(
            path,
            line,
            &format!("invalid numeric argument `{argument}` for {kind}: {error}"),
        )
    })
}

/// Push one step-owned case into the metadata accumulator.
fn push_step_case(metadata: &mut FixtureMetadata, step_index: usize, step_case: LspStepCase) {
    metadata
        .step_cases
        .entry(step_index)
        .or_default()
        .push(step_case);
}

/// Push one marker-owned empty step case.
fn push_marker_empty_step_case<F>(
    path: &Path,
    line: usize,
    parts: &[&str],
    step_index: usize,
    metadata: &mut FixtureMetadata,
    build_step_case: F,
) -> Result<(), FixtureParseError>
where
    F: FnOnce(String) -> LspStepCase,
{
    let Some(marker_name) = parts.get(2) else {
        return Err(parse_error(
            path,
            line,
            "expected `lsp <kind> <marker> [step]`",
        ));
    };

    let marker_name = (*marker_name).to_string();
    let step_case = build_step_case(marker_name);

    push_step_case(metadata, step_index, step_case);

    Ok(())
}

/// Push one marker-owned snapshot step case.
fn push_marker_snapshot_step_case<F>(
    path: &Path,
    line: usize,
    parts: &[&str],
    step_index: usize,
    metadata: &mut FixtureMetadata,
    raw_body: &str,
    build_step_case: F,
) -> Result<(), FixtureParseError>
where
    F: FnOnce(String, String) -> LspStepCase,
{
    let Some(marker_name) = parts.get(2) else {
        return Err(parse_error(
            path,
            line,
            "expected `lsp <kind> <marker> [step]`",
        ));
    };

    let marker_name = (*marker_name).to_string();
    let snapshot_text = normalize_snapshot_text(raw_body);
    let step_case = build_step_case(marker_name, snapshot_text);

    push_step_case(metadata, step_index, step_case);

    Ok(())
}

/// Push one file-owned snapshot step case.
fn push_file_snapshot_step_case<F>(
    path: &Path,
    line: usize,
    parts: &[&str],
    step_index: usize,
    metadata: &mut FixtureMetadata,
    raw_body: &str,
    build_step_case: F,
) -> Result<(), FixtureParseError>
where
    F: FnOnce(String, String) -> LspStepCase,
{
    let Some(file_path) = parts.get(2) else {
        return Err(parse_error(
            path,
            line,
            "expected `lsp <kind> <path> [step]`",
        ));
    };

    let file_path = (*file_path).to_string();
    let snapshot_text = normalize_snapshot_text(raw_body);
    let step_case = build_step_case(file_path, snapshot_text);

    push_step_case(metadata, step_index, step_case);

    Ok(())
}

/// Push one file-owned text step case.
fn push_file_text_step_case<F>(
    path: &Path,
    line: usize,
    parts: &[&str],
    step_index: usize,
    metadata: &mut FixtureMetadata,
    raw_body: &str,
    build_step_case: F,
) -> Result<(), FixtureParseError>
where
    F: FnOnce(String, String) -> LspStepCase,
{
    let Some(file_path) = parts.get(2) else {
        return Err(parse_error(
            path,
            line,
            "expected `lsp <kind> <path> [step]`",
        ));
    };

    let file_path = (*file_path).to_string();
    let text = normalize_snapshot_text(raw_body);
    let step_case = build_step_case(file_path, text);

    push_step_case(metadata, step_index, step_case);

    Ok(())
}

/// Push one query-owned snapshot step case.
fn push_query_snapshot_step_case<F>(
    path: &Path,
    line: usize,
    parts: &[&str],
    step_index: usize,
    metadata: &mut FixtureMetadata,
    raw_body: &str,
    build_step_case: F,
) -> Result<(), FixtureParseError>
where
    F: FnOnce(String, String) -> LspStepCase,
{
    let Some(query) = parts.get(2) else {
        return Err(parse_error(
            path,
            line,
            "expected `lsp <kind> <query> [step]`",
        ));
    };

    let query = (*query).to_string();
    let snapshot_text = normalize_snapshot_text(raw_body);
    let step_case = build_step_case(query, snapshot_text);

    push_step_case(metadata, step_index, step_case);

    Ok(())
}

/// Take one trailing labeled step suffix from one `lsp ...` block language tag.
fn take_step_suffix(
    parts: &mut Vec<&str>,
    path: &Path,
    line: usize,
) -> Result<Option<usize>, FixtureParseError> {
    let Some(last_part) = parts.last().copied() else {
        return Ok(None);
    };

    // plain blocks stay on the non-step path
    if !last_part.starts_with('[') || !last_part.ends_with(']') {
        return Ok(None);
    }

    let step_index = parse_step_label(path, line, last_part)?;
    parts.pop();

    Ok(Some(step_index))
}

/// Parse one optional step suffix from one mdtest source file path.
fn parse_versioned_file_path(
    path: &Path,
    line: usize,
    file_path: &str,
) -> Result<(String, usize), FixtureParseError> {
    let Some(open_bracket) = file_path.rfind('[') else {
        return Ok((file_path.to_string(), 0));
    };

    // plain file paths without one trailing step index stay at the base snapshot
    if !file_path.ends_with(']') {
        return Ok((file_path.to_string(), 0));
    }

    let base_path = &file_path[..open_bracket];
    let label = &file_path[open_bracket..];
    let step_index = parse_step_label(path, line, label)?;

    Ok((base_path.to_string(), step_index))
}

/// Parse one numeric step reference like `[0]` or `[2]`.
fn parse_step_label(path: &Path, line: usize, label: &str) -> Result<usize, FixtureParseError> {
    let Some(label) = label.strip_prefix('[') else {
        return Err(parse_error(
            path,
            line,
            "expected step index to start with `[`",
        ));
    };
    let Some(label) = label.strip_suffix(']') else {
        return Err(parse_error(
            path,
            line,
            "expected step index to end with `]`",
        ));
    };

    let label = label.trim();
    if label.is_empty() {
        return Err(parse_error(path, line, "expected one numeric step index"));
    }

    let step_index = label
        .parse::<usize>()
        .map_err(|_| parse_error(path, line, "expected one numeric step index"))?;

    Ok(step_index)
}

/// Parse one singleton scalar expectation block when the kind matches.
fn parse_scalar_block(
    path: &Path,
    line: usize,
    kind: &str,
    body: &str,
    metadata: &mut FixtureMetadata,
) -> Result<bool, FixtureParseError> {
    match kind {
        "completion_resolve_label" => set_text_block(
            path,
            line,
            "completion_resolve_label",
            &mut metadata.expectations.completion.resolve_label,
            body,
        )?,
        "completion_resolve_documentation" => set_text_block(
            path,
            line,
            "completion_resolve_documentation",
            &mut metadata.expectations.completion.resolve_documentation,
            body,
        )?,
        "hover_signature" => set_text_block(
            path,
            line,
            "hover_signature",
            &mut metadata.expectations.hover.signature,
            body,
        )?,
        "hover_documentation" => set_text_block(
            path,
            line,
            "hover_documentation",
            &mut metadata.expectations.hover.documentation,
            body,
        )?,
        "signature_label" => set_text_block(
            path,
            line,
            "signature_label",
            &mut metadata.expectations.signature_help.label,
            body,
        )?,
        "workspace_symbol_query" => set_text_block(
            path,
            line,
            "workspace_symbol_query",
            &mut metadata.expectations.symbols.workspace_query,
            body,
        )?,

        // parse numeric singleton values explicitly so invalid numbers fail loudly
        "active_parameter" => {
            let parameter = body.parse::<usize>().map_err(|error| FixtureParseError {
                path: path.to_path_buf(),
                line,
                message: format!("invalid active parameter index: {error}"),
            })?;

            set_value_block(
                path,
                line,
                "active_parameter",
                &mut metadata.expectations.signature_help.active_parameter,
                parameter,
            )?;
        }

        _ => return Ok(false),
    }

    Ok(true)
}

/// Parse one singleton snapshot expectation block when the kind matches.
fn parse_snapshot_block_kind(
    path: &Path,
    line: usize,
    kind: &str,
    raw_body: &str,
    metadata: &mut FixtureMetadata,
) -> Result<bool, FixtureParseError> {
    match kind {
        "document_formatting" => set_snapshot_block(
            path,
            line,
            "document_formatting",
            &mut metadata.expectations.formatting.document_expected_text,
            raw_body,
        )?,
        "range_formatting" => set_snapshot_block(
            path,
            line,
            "range_formatting",
            &mut metadata.expectations.formatting.range_expected_text,
            raw_body,
        )?,
        "code_action_result" => set_snapshot_block(
            path,
            line,
            "code_action_result",
            &mut metadata.expectations.code_actions.expected_text,
            raw_body,
        )?,
        "semantic_tokens" => set_snapshot_block(
            path,
            line,
            "semantic_tokens",
            &mut metadata.expectations.semantic_tokens.full_expected_text,
            raw_body,
        )?,
        "semantic_tokens_range" => set_snapshot_block(
            path,
            line,
            "semantic_tokens_range",
            &mut metadata.expectations.semantic_tokens.range_expected_text,
            raw_body,
        )?,
        "semantic_tokens_delta" => set_snapshot_block(
            path,
            line,
            "semantic_tokens_delta",
            &mut metadata.expectations.semantic_tokens.delta_expected_text,
            raw_body,
        )?,
        "document_link" => set_snapshot_block(
            path,
            line,
            "document_link",
            &mut metadata.expectations.document_links.snapshot_text,
            raw_body,
        )?,
        "folding_range" => set_snapshot_block(
            path,
            line,
            "folding_range",
            &mut metadata.expectations.folding_ranges.snapshot_text,
            raw_body,
        )?,
        "inlay_hint" => set_snapshot_block(
            path,
            line,
            "inlay_hint",
            &mut metadata.expectations.inlay_hints.snapshot_text,
            raw_body,
        )?,
        "code_lens" => set_snapshot_block(
            path,
            line,
            "code_lens",
            &mut metadata.expectations.code_lenses.snapshot_text,
            raw_body,
        )?,
        "call_hierarchy_incoming" => set_snapshot_block(
            path,
            line,
            "call_hierarchy_incoming",
            &mut metadata.expectations.hierarchy.call_incoming_text,
            raw_body,
        )?,
        "call_hierarchy_outgoing" => set_snapshot_block(
            path,
            line,
            "call_hierarchy_outgoing",
            &mut metadata.expectations.hierarchy.call_outgoing_text,
            raw_body,
        )?,
        "type_hierarchy_supertypes" => set_snapshot_block(
            path,
            line,
            "type_hierarchy_supertypes",
            &mut metadata.expectations.hierarchy.type_supertypes_text,
            raw_body,
        )?,
        "type_hierarchy_subtypes" => set_snapshot_block(
            path,
            line,
            "type_hierarchy_subtypes",
            &mut metadata.expectations.hierarchy.type_subtypes_text,
            raw_body,
        )?,
        "workspace_diagnostic" => set_snapshot_block(
            path,
            line,
            "workspace_diagnostic",
            &mut metadata.expectations.diagnostics.workspace_text,
            raw_body,
        )?,
        "workspace_diagnostic_partial" => set_snapshot_block(
            path,
            line,
            "workspace_diagnostic_partial",
            &mut metadata.expectations.diagnostics.workspace_partial_text,
            raw_body,
        )?,
        "selection_range" => set_snapshot_block(
            path,
            line,
            "selection_range",
            &mut metadata.expectations.diagnostics.selection_range_text,
            raw_body,
        )?,

        // keep one shared current-file snapshot for both edit and formatting flows
        "current_file" => {
            let text = normalize_snapshot_text(raw_body);

            set_value_block(
                path,
                line,
                "current_file",
                &mut metadata.expectations.edits.current_file_text,
                text.clone(),
            )?;

            set_value_block(
                path,
                line,
                "current_file",
                &mut metadata.expectations.formatting.current_file_text,
                text,
            )?;
        }

        _ => return Ok(false),
    }

    Ok(true)
}

/// Parse one source-backed block or feature flag when the kind matches.
fn parse_source_block_kind(
    path: &Path,
    line: usize,
    kind: &str,
    raw_body: &str,
    body: &str,
    parts: &[&str],
    metadata: &mut FixtureMetadata,
) -> Result<bool, FixtureParseError> {
    match kind {
        "execute_command" => {
            let Some(command) = parts.get(2) else {
                return Err(parse_error(
                    path,
                    line,
                    "expected `lsp execute_command <command>`",
                ));
            };

            if !body.is_empty() {
                return Err(parse_error(
                    path,
                    line,
                    "lsp execute_command blocks must be empty",
                ));
            }

            metadata
                .expectations
                .commands
                .execute
                .push((*command).to_string());
        }
        "semantic_tokens_delta_source" => set_source_block(
            path,
            line,
            "semantic_tokens_delta_source",
            &mut metadata.expectations.semantic_tokens.delta_source_text,
            "semantic_tokens_delta_source",
            raw_body,
        )?,
        "code_action_resolve_support" => {
            // reject duplicate feature flag blocks loudly
            if metadata.code_action_resolve_support_is_set {
                return Err(parse_error(
                    path,
                    line,
                    "duplicate lsp code_action_resolve_support block",
                ));
            }

            metadata.expectations.code_actions.enable_resolve_support =
                parse_bool(path, line, kind, body)?;
            metadata.code_action_resolve_support_is_set = true;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Parse one repeated expectation block when the kind matches.
fn parse_repeated_block(
    path: &Path,
    line: usize,
    kind: &str,
    body: &str,
    metadata: &mut FixtureMetadata,
) -> Result<bool, FixtureParseError> {
    match kind {
        "document_symbol" => {
            let symbols = parse_expected_document_symbols(body)
                .map_err(|message| parse_error(path, line, &message))?;
            metadata.expectations.symbols.document.extend(symbols);
        }
        "workspace_symbol" => {
            let symbols = parse_expected_workspace_symbols(body)
                .map_err(|message| parse_error(path, line, &message))?;
            metadata.expectations.symbols.workspace.extend(symbols);
        }
        "completion_item" => {
            let items = parse_expected_completion_items(body)
                .map_err(|message| parse_error(path, line, &message))?;
            metadata.expectations.completion.items.extend(items);
        }
        "code_action" => {
            let actions = parse_expected_code_actions(body)
                .map_err(|message| parse_error(path, line, &message))?;
            metadata.expectations.code_actions.actions.extend(actions);
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Set one singleton text block, rejecting duplicates loudly.
fn set_text_block(
    path: &Path,
    line: usize,
    kind: &str,
    slot: &mut Option<String>,
    value: &str,
) -> Result<(), FixtureParseError> {
    set_value_block(path, line, kind, slot, value.to_string())
}

/// Set one singleton snapshot block, rejecting duplicates loudly.
fn set_snapshot_block(
    path: &Path,
    line: usize,
    kind: &str,
    slot: &mut Option<String>,
    value: &str,
) -> Result<(), FixtureParseError> {
    set_value_block(path, line, kind, slot, normalize_snapshot_text(value))
}

/// Set one singleton source block, rejecting duplicates loudly.
fn set_source_block(
    path: &Path,
    line: usize,
    kind: &str,
    slot: &mut Option<String>,
    file_path: &str,
    value: &str,
) -> Result<(), FixtureParseError> {
    let text = normalize_source_text(path, file_path, value)?;

    set_value_block(path, line, kind, slot, text)
}

/// Set one singleton value, rejecting duplicates loudly.
fn set_value_block<T: Clone>(
    path: &Path,
    line: usize,
    kind: &str,
    slot: &mut Option<T>,
    value: T,
) -> Result<(), FixtureParseError> {
    if slot.is_some() {
        return Err(parse_error(
            path,
            line,
            &format!("duplicate lsp {kind} block"),
        ));
    }

    *slot = Some(value);

    Ok(())
}

/// Build one fixture parse error with path and line context.
fn parse_error(path: &Path, line: usize, message: &str) -> FixtureParseError {
    FixtureParseError {
        path: path.to_path_buf(),
        line,
        message: message.to_string(),
    }
}

/// Parse one required boolean value.
fn parse_bool(
    path: &Path,
    line: usize,
    label: &str,
    value: &str,
) -> Result<bool, FixtureParseError> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(parse_error(
            path,
            line,
            &format!("expected {label} to be true or false"),
        )),
    }
}

/// Normalize one inline snapshot block body.
fn normalize_snapshot_text(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

/// Normalize one inline edit-step body.
fn normalize_step_text(text: &str) -> String {
    let mut text = normalize_snapshot_text(text);

    // markdown code fences always contribute one terminal newline:
    // drop that implicit newline and let a blank trailing line encode one real newline
    if text.ends_with('\n') {
        text.pop();
    }

    text
}

/// Normalize one inline source block body by stripping fixture markers and ranges.
fn normalize_source_text(
    path: &Path,
    file_path: &str,
    text: &str,
) -> Result<String, FixtureParseError> {
    let parsed = parse_file_text(
        path,
        &MdTestFile {
            path: file_path.to_string(),
            content: normalize_snapshot_text(text),
            options: Default::default(),
        },
    )?;

    Ok(parsed.text)
}

/// Parse one fixture file into stripped text, markers, and ranges.
fn parse_file_text(path: &Path, file: &MdTestFile) -> Result<ParsedFileText, FixtureParseError> {
    let bytes = file.content.as_bytes();
    let mut output = String::with_capacity(file.content.len());
    let mut markers = Vec::<Marker>::new();
    let mut range_stack = Vec::<usize>::new();
    let mut ranges = Vec::<RangeOffset>::new();
    let mut index = 0usize;

    // strip marker and range delimiters while recording offsets
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            let Some(comment_end) = find_subslice(&bytes[index + 2..], b"*/") else {
                return Err(parse_error(path, 1, "unterminated marker comment"));
            };
            let marker_name = &file.content[index + 2..index + 2 + comment_end];
            markers.push(Marker {
                name: marker_name.to_string(),
                file_path: file.path.clone(),
                offset: output.len(),
                line: 0,
                character: 0,
            });
            index += comment_end + 4;
            continue;
        }

        if bytes[index..].starts_with(b"[|") {
            range_stack.push(output.len());
            index += 2;
            continue;
        }

        if bytes[index..].starts_with(b"|]") {
            let Some(start_offset) = range_stack.pop() else {
                return Err(parse_error(path, 1, "found |] without matching [|"));
            };

            ranges.push(RangeOffset {
                file_path: file.path.clone(),
                start_offset,
                end_offset: output.len(),
            });
            index += 2;
            continue;
        }

        let character = file.content[index..]
            .chars()
            .next()
            .expect("index always points to one valid char boundary");
        output.push(character);
        index += character.len_utf8();
    }

    // reject unterminated nested ranges loudly
    if !range_stack.is_empty() {
        return Err(parse_error(path, 1, "unterminated [| range"));
    }

    Ok(ParsedFileText {
        text: output,
        markers,
        ranges,
    })
}

/// Find one subslice inside one byte slice.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Compute the byte offset for each line start in one stripped file.
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut line_starts = vec![0usize];

    // record byte offsets after each newline so later lookups stay cheap
    for (offset, character) in text.char_indices() {
        if character == '\n' {
            line_starts.push(offset + 1);
        }
    }

    line_starts
}

/// Convert one stripped byte offset into line and character indices.
fn offset_to_line_character(line_starts: &[usize], offset: usize) -> (usize, usize) {
    let line_index = line_starts.partition_point(|line_start| *line_start <= offset) - 1;
    let line_start = line_starts[line_index];
    let character = offset - line_start;

    (line_index, character)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::lsp::fixture::parse::parse_mdtest_case;
    use crate::mdtest::{MdTestCase, MdTestFile, RawCodeBlock};

    #[test]
    fn test_parse_mdtest_case_with_step_blocks_and_markers() {
        let test = MdTestCase {
            name: "Quick Info Exists".to_string(),
            section: "Navigation".to_string(),
            options: HashMap::new(),
            files: vec![MdTestFile {
                path: "main.ds".to_string(),
                content: "const /*hover_exists*/value = 1;\n".to_string(),
                options: HashMap::new(),
            }],
            bullet_items: Vec::new(),
            extra_blocks: vec![RawCodeBlock {
                language: "lsp quick_info_exists hover_exists [0]".to_string(),
                content: String::new(),
            }],
            line: 1,
            skip: false,
        };

        let fixture = parse_mdtest_case(Path::new("fixture.md"), &test).expect("fixture");

        assert_eq!(fixture.step_indices(), vec![0]);
        assert_eq!(fixture.step_cases(0).len(), 1);
        assert_eq!(fixture.markers.len(), 1);
        assert_eq!(fixture.marker("hover_exists").unwrap().offset, 6);
        assert_eq!(fixture.files.len(), 1);
    }

    #[test]
    fn test_parse_inline_snapshot_blocks() {
        let test = MdTestCase {
            name: "Current File".to_string(),
            section: "Lifecycle".to_string(),
            options: HashMap::new(),
            files: vec![MdTestFile {
                path: "main.ds".to_string(),
                content: "const value = 1;\n".to_string(),
                options: HashMap::new(),
            }],
            bullet_items: Vec::new(),
            extra_blocks: vec![RawCodeBlock {
                language: "lsp current_file".to_string(),
                content: "const value = 2;\n".to_string(),
            }],
            line: 1,
            skip: false,
        };

        let fixture = parse_mdtest_case(Path::new("fixture.md"), &test).expect("fixture");

        assert_eq!(fixture.files.len(), 1);
        assert_eq!(
            fixture.expectations.edits.current_file_text.as_deref(),
            Some("const value = 2;\n")
        );
    }

    #[test]
    fn test_reject_duplicate_singleton_blocks() {
        let test = MdTestCase {
            name: "Duplicate Current File".to_string(),
            section: "Lifecycle".to_string(),
            options: HashMap::new(),
            files: vec![MdTestFile {
                path: "main.ds".to_string(),
                content: "const value = 1;\n".to_string(),
                options: HashMap::new(),
            }],
            bullet_items: Vec::new(),
            extra_blocks: vec![
                RawCodeBlock {
                    language: "lsp current_file".to_string(),
                    content: "const value = 2;\n".to_string(),
                },
                RawCodeBlock {
                    language: "lsp current_file".to_string(),
                    content: "const value = 3;\n".to_string(),
                },
            ],
            line: 1,
            skip: false,
        };

        let error = parse_mdtest_case(Path::new("fixture.md"), &test).unwrap_err();

        assert_eq!(error.message, "duplicate lsp current_file block");
    }
}
