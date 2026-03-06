use std::collections::BTreeMap;
use std::path::Path;

use crate::lsp::fixture::{
    FixtureParseError, LspExpectations, LspFixture, LspScenario, LspSourceFile, Marker, Range,
    parse_expected_code_actions, parse_expected_completion_items, parse_expected_document_symbols,
    parse_expected_workspace_symbols,
};
use crate::mdtest::{MdTestCase, MdTestFile, RawCodeBlock};

const LSP_BLOCK_PREFIX: &str = "lsp";

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenTextOverride {
    /// The file path that should be opened.
    target_file_path: String,
    /// The overlay text that should be opened into that file.
    overlay_text: String,
}

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
    /// The bespoke scenarios declared by fixture metadata.
    scenarios: Vec<LspScenario>,
    /// The grouped expectations declared by fixture metadata.
    expectations: LspExpectations,
    /// The open-text overrides declared by fixture metadata.
    open_text_overrides: Vec<OpenTextOverride>,
    /// Whether the code-action resolve support flag has been set.
    code_action_resolve_support_is_set: bool,
}

impl FixtureMetadata {
    /// Build one empty metadata accumulator.
    fn new() -> Self {
        Self {
            scenarios: Vec::new(),
            expectations: LspExpectations::default(),
            open_text_overrides: Vec::new(),
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

    let open_text_overrides = metadata
        .open_text_overrides
        .iter()
        .map(|entry| (entry.target_file_path.clone(), entry.overlay_text.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut parsed_files = Vec::<LspSourceFile>::new();
    let mut markers = BTreeMap::<String, Marker>::new();
    let mut ranges = Vec::<Range>::new();

    // parse fixture files with marker stripping and keep source text exact
    for file in &test.files {
        let parsed_file = parse_file_text(path, file)?;
        let line_starts = compute_line_starts(&parsed_file.text);

        for marker in parsed_file.markers {
            if markers.contains_key(&marker.name) {
                return Err(FixtureParseError {
                    path: path.to_path_buf(),
                    line: test.line,
                    message: format!("duplicate marker name {}", marker.name),
                });
            }

            let (line, character) = offset_to_line_character(&line_starts, marker.offset);
            markers.insert(
                marker.name.clone(),
                Marker {
                    line,
                    character,
                    ..marker
                },
            );
        }

        for range in parsed_file.ranges {
            let (start_line, start_character) =
                offset_to_line_character(&line_starts, range.start_offset);
            let (end_line, end_character) =
                offset_to_line_character(&line_starts, range.end_offset);
            let text = parsed_file.text[range.start_offset..range.end_offset].to_string();

            ranges.push(Range {
                file_path: range.file_path,
                start_offset: range.start_offset,
                end_offset: range.end_offset,
                start_line,
                start_character,
                end_line,
                end_character,
                text,
            });
        }

        parsed_files.push(LspSourceFile {
            path: file.path.clone(),
            text: parsed_file.text,
        });
    }

    // require at least one file so empty fixtures fail loudly
    if parsed_files.is_empty() {
        return Err(parse_error(
            path,
            test.line,
            "fixture did not declare any files",
        ));
    }

    Ok(LspFixture {
        path: path.to_path_buf(),
        files: parsed_files,
        markers,
        ranges,
        scenarios: metadata.scenarios,
        expectations: metadata.expectations,
        open_text_overrides,
    })
}

/// Parse one `lsp ...` block and update the grouped fixture metadata.
fn parse_metadata_block(
    path: &Path,
    line: usize,
    block: &RawCodeBlock,
    metadata: &mut FixtureMetadata,
) -> Result<(), FixtureParseError> {
    let parts = block.language.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() || parts[0] != LSP_BLOCK_PREFIX {
        return Ok(());
    }

    // reject malformed empty lsp blocks loudly
    if parts.len() < 2 {
        return Err(parse_error(path, line, "expected `lsp <kind> ...` block"));
    }

    let kind = parts[1];
    let raw_body = block.content.as_str();
    let body = raw_body.trim_end();

    // scenarios
    if kind == "scenario" {
        return parse_scenario_block(path, line, &parts, metadata);
    }

    // singleton scalar expectations
    if parse_scalar_block(path, line, kind, body, metadata)? {
        return Ok(());
    }

    // singleton snapshot expectations
    if parse_snapshot_block_kind(path, line, kind, raw_body, metadata)? {
        return Ok(());
    }

    // source overlays and feature flags
    if parse_source_block_kind(path, line, kind, raw_body, body, &parts, metadata)? {
        return Ok(());
    }

    // repeated line-oriented expectations
    if parse_repeated_block(path, line, kind, body, metadata)? {
        return Ok(());
    }

    Err(parse_error(
        path,
        line,
        &format!("unknown lsp block kind {kind}"),
    ))
}

/// Parse one scenario metadata block.
fn parse_scenario_block(
    path: &Path,
    line: usize,
    parts: &[&str],
    metadata: &mut FixtureMetadata,
) -> Result<(), FixtureParseError> {
    let Some(name) = parts.get(2) else {
        return Err(parse_error(path, line, "expected `lsp scenario <name>`"));
    };

    let scenario = LspScenario::parse(name)
        .ok_or_else(|| parse_error(path, line, &format!("unknown scenario {name}")))?;

    push_scenario(path, line, scenario, &mut metadata.scenarios)
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
        "open_text" => {
            let Some(target_file_path) = parts.get(2) else {
                return Err(parse_error(path, line, "expected `lsp open_text <target>`"));
            };

            push_open_text_override(path, line, &metadata.open_text_overrides, target_file_path)?;
            metadata.open_text_overrides.push(OpenTextOverride {
                target_file_path: (*target_file_path).to_string(),
                overlay_text: normalize_source_text(path, target_file_path, raw_body)?,
            });
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

/// Push one scenario, rejecting duplicates loudly.
fn push_scenario(
    path: &Path,
    line: usize,
    scenario: LspScenario,
    scenarios: &mut Vec<LspScenario>,
) -> Result<(), FixtureParseError> {
    if scenarios.contains(&scenario) {
        return Err(parse_error(path, line, "duplicate lsp scenario block"));
    }

    scenarios.push(scenario);

    Ok(())
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

/// Reject duplicate open-text overrides for the same file.
fn push_open_text_override(
    path: &Path,
    line: usize,
    overrides: &[OpenTextOverride],
    target_file_path: &str,
) -> Result<(), FixtureParseError> {
    if overrides
        .iter()
        .any(|entry| entry.target_file_path == target_file_path)
    {
        return Err(parse_error(
            path,
            line,
            &format!("duplicate lsp open_text block for {target_file_path}"),
        ));
    }

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

    use crate::lsp::LspScenario;
    use crate::lsp::fixture::parse::parse_mdtest_case;
    use crate::mdtest::{MdTestCase, MdTestFile, RawCodeBlock};

    #[test]
    fn test_parse_mdtest_case_with_scenario_and_markers() {
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
                language: "lsp scenario quick-info-exists".to_string(),
                content: String::new(),
            }],
            line: 1,
            skip: false,
        };

        let fixture = parse_mdtest_case(Path::new("fixture.md"), &test).expect("fixture");

        assert_eq!(fixture.scenarios, vec![LspScenario::QuickInfoExists]);
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
            extra_blocks: vec![
                RawCodeBlock {
                    language: "lsp current_file".to_string(),
                    content: "const value = 2;\n".to_string(),
                },
                RawCodeBlock {
                    language: "lsp open_text main.ds".to_string(),
                    content: "const value = 3;\n".to_string(),
                },
            ],
            line: 1,
            skip: false,
        };

        let fixture = parse_mdtest_case(Path::new("fixture.md"), &test).expect("fixture");

        assert_eq!(fixture.files.len(), 1);
        assert_eq!(
            fixture.expectations.edits.current_file_text.as_deref(),
            Some("const value = 2;\n")
        );
        assert_eq!(fixture.open_text_overrides["main.ds"], "const value = 3;\n");
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

    #[test]
    fn test_reject_duplicate_open_text_blocks() {
        let test = MdTestCase {
            name: "Duplicate Open Text".to_string(),
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
                    language: "lsp open_text main.ds".to_string(),
                    content: "const value = 2;\n".to_string(),
                },
                RawCodeBlock {
                    language: "lsp open_text main.ds".to_string(),
                    content: "const value = 3;\n".to_string(),
                },
            ],
            line: 1,
            skip: false,
        };

        let error = parse_mdtest_case(Path::new("fixture.md"), &test).unwrap_err();

        assert_eq!(error.message, "duplicate lsp open_text block for main.ds");
    }
}
