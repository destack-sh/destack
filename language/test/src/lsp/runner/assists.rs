use crate::lsp::runner::range_for_marker;
use crate::lsp::{
    LspFixture, LspTestState, NormalizedHover, NormalizedLocation,
    NormalizedResolvedCompletionItem, NormalizedSignatureHelp, NormalizedSignatureInformation,
    normalize_code_lenses, normalize_folding_ranges, normalize_hover, normalize_inlay_hints,
    normalize_prepare_rename, normalize_resolved_completion_item, normalize_signature_help,
    verify_code_lenses, verify_definition_locations, verify_folding_ranges, verify_hover,
    verify_inlay_hints, verify_resolved_completion_item, verify_signature_help,
};

/// Run the assist-like cases declared by one fixture.
pub(crate) fn run_assist_cases(
    fixture: &LspFixture,
    test_state: &mut LspTestState,
) -> Result<(), String> {
    // hover
    if test_state.marker("hover").is_some() {
        let hover_marker = fixture
            .marker("hover")
            .ok_or_else(|| "hover fixture is missing /*hover*/ marker".to_string())?;
        let signature = fixture
            .expectations
            .hover
            .signature
            .as_deref()
            .ok_or_else(|| "hover fixture is missing @HoverSignature".to_string())?;
        let hover_range = range_for_marker(fixture, hover_marker)
            .ok_or_else(|| "hover fixture is missing range around /*hover*/".to_string())?;
        let expected_hover = NormalizedHover {
            contents_kind: "markdown",
            contents: expected_hover_markdown(
                signature,
                fixture.expectations.hover.documentation.as_deref(),
                &hover_marker.file_path,
                hover_marker.line,
                hover_marker.character,
            ),
            range: Some(NormalizedLocation {
                file_path: hover_range.file_path.clone(),
                start_line: hover_range.start_line,
                start_character: hover_range.start_character,
                end_line: hover_range.end_line,
                end_character: hover_range.end_character,
            }),
        };

        test_state.go_to_marker("hover")?;
        let hover = test_state
            .request_hover()?
            .ok_or_else(|| "expected hover result".to_string())?;
        let file_path = test_state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "hover request did not keep an active file".to_string())?;
        let actual_hover = normalize_hover(test_state.workspace_root(), &file_path, &hover)?;

        verify_hover(&actual_hover, &expected_hover)?;
        test_state.verify().quick_info_is(
            signature,
            fixture.expectations.hover.documentation.as_deref(),
        )?;
    }

    // signature help
    if test_state.marker("signature").is_some() {
        let signature_label = fixture
            .expectations
            .signature_help
            .label
            .as_deref()
            .ok_or_else(|| "signature help fixture is missing @SignatureLabel".to_string())?;
        let active_parameter = fixture
            .expectations
            .signature_help
            .active_parameter
            .ok_or_else(|| "signature help fixture is missing @ActiveParameter".to_string())?;
        let expected_help = NormalizedSignatureHelp {
            signatures: vec![NormalizedSignatureInformation {
                label: signature_label.to_string(),
                parameters: parameter_labels_from_signature(signature_label),
            }],
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        };

        test_state.go_to_marker("signature")?;
        let help = test_state
            .request_signature_help()?
            .ok_or_else(|| "expected signature help result".to_string())?;
        let actual_help = normalize_signature_help(&help);

        verify_signature_help(&actual_help, &expected_help)?;
        test_state.verify().signature_help(&expected_help)?;
    }

    // prepare rename
    if test_state.marker("prepare_rename").is_some() {
        let rename_marker = fixture.marker("prepare_rename").ok_or_else(|| {
            "prepare rename fixture is missing /*prepare_rename*/ marker".to_string()
        })?;
        let expected_range = range_for_marker(fixture, rename_marker)
            .ok_or_else(|| "prepare rename fixture is missing range around marker".to_string())?;
        let expected_location = NormalizedLocation {
            file_path: expected_range.file_path.clone(),
            start_line: expected_range.start_line,
            start_character: expected_range.start_character,
            end_line: expected_range.end_line,
            end_character: expected_range.end_character,
        };

        test_state.go_to_marker("prepare_rename")?;
        let response = test_state
            .request_prepare_rename()?
            .ok_or_else(|| "expected prepare rename result".to_string())?;
        let active_file_path = test_state
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "prepare rename request did not keep an active file".to_string())?;
        let actual_location = normalize_prepare_rename(&active_file_path, &response)?;

        verify_definition_locations(&[actual_location], &[expected_location])?;
    }

    // completion
    if !fixture.expectations.completion.items.is_empty() {
        test_state.go_to_marker("completion")?;
        let expected_completion_pairs = fixture
            .expectations
            .completion
            .items
            .iter()
            .map(|item| {
                let kind = normalize_expected_completion_kind(&item.kind)?;

                Ok::<(&str, &str), String>((item.label.as_str(), kind))
            })
            .collect::<Result<Vec<_>, String>>()?;

        test_state
            .verify()
            .completions(&expected_completion_pairs)?;
    }

    // completion resolve
    if let Some(expected_label) = fixture.expectations.completion.resolve_label.as_deref() {
        let expected_documentation = fixture
            .expectations
            .completion
            .resolve_documentation
            .clone();
        let expected_resolved = NormalizedResolvedCompletionItem {
            label: expected_label.to_string(),
            documentation: expected_documentation,
        };

        test_state.go_to_marker("completion")?;
        let completion = test_state
            .request_completion()?
            .ok_or_else(|| "expected completion result".to_string())?;
        let resolved_item = completion_item_by_label(completion, expected_label)?
            .ok_or_else(|| format!("failed to find completion item {expected_label}"))?;
        let resolved_item = test_state
            .resolve_completion(resolved_item)?
            .ok_or_else(|| "expected completion resolve result".to_string())?;
        let actual_resolved = normalize_resolved_completion_item(&resolved_item)?;

        verify_resolved_completion_item(&actual_resolved, &expected_resolved)?;
    }

    // inlay hints
    if let Some(expected_snapshot) = fixture.expectations.inlay_hints.snapshot_text.as_deref() {
        let expected_hints = crate::lsp::parse_expected_inlay_hints(expected_snapshot)?;
        let active_file_path = fixture.first_file_path()?;

        test_state.go_to().file(&active_file_path)?;
        let hints = test_state
            .request_inlay_hints()?
            .ok_or_else(|| "expected inlay hints result".to_string())?;
        let actual_hints = normalize_inlay_hints(&hints)?;

        verify_inlay_hints(&actual_hints, &expected_hints)?;
    }

    // folding ranges
    if let Some(expected_snapshot) = fixture.expectations.folding_ranges.snapshot_text.as_deref() {
        let expected_ranges = crate::lsp::parse_expected_folding_ranges(expected_snapshot)?;
        let active_file_path = fixture.first_file_path()?;

        test_state.go_to().file(&active_file_path)?;
        let ranges = test_state
            .request_folding_ranges()?
            .ok_or_else(|| "expected folding ranges result".to_string())?;
        let actual_ranges = normalize_folding_ranges(&ranges);

        verify_folding_ranges(&actual_ranges, &expected_ranges)?;
    }

    // code lenses
    if let Some(expected_snapshot) = fixture.expectations.code_lenses.snapshot_text.as_deref() {
        let active_file_path = fixture.first_file_path()?;
        let expected_lenses = crate::lsp::parse_expected_code_lenses(expected_snapshot)?;

        test_state.go_to().file(&active_file_path)?;
        let lenses = test_state
            .request_code_lenses()?
            .ok_or_else(|| "expected code lenses result".to_string())?;
        let actual_lenses = normalize_code_lenses(&lenses);

        verify_code_lenses(&actual_lenses, &expected_lenses)?;

        // resolve the first eager lens to keep the no-op resolve path covered
        if let Some(first_lens) = lenses.into_iter().next() {
            let resolved_lens = test_state
                .resolve_code_lens(first_lens)?
                .ok_or_else(|| "expected code lens resolve result".to_string())?;
            let actual_resolved = normalize_code_lenses(&[resolved_lens]);
            let expected_resolved = expected_lenses
                .first()
                .cloned()
                .ok_or_else(|| "code lens fixture is missing expected lens".to_string())?;

            verify_code_lenses(&actual_resolved, &[expected_resolved])?;
        }
    }

    Ok(())
}

/// Return one completion item with a unique matching label.
fn completion_item_by_label(
    completion: destack_lsp_types::CompletionResponse,
    label: &str,
) -> Result<Option<destack_lsp_types::CompletionItem>, String> {
    let items = match completion {
        destack_lsp_types::CompletionResponse::Array(items) => items,
        destack_lsp_types::CompletionResponse::List(list) => list.items,
    };

    // require one unique match so resolve fixtures cannot silently bind to the wrong item
    let mut matches = items.into_iter().filter(|item| item.label == label);
    let first = matches.next();

    if matches.next().is_some() {
        return Err(format!(
            "completion resolve fixture matched multiple completion items with label {label}"
        ));
    }

    Ok(first)
}

/// Build the exact markdown hover payload for one signature fixture.
fn expected_hover_markdown(
    signature: &str,
    documentation: Option<&str>,
    file_path: &str,
    line: usize,
    character: usize,
) -> String {
    let mut markdown = String::new();
    markdown.push_str("**Signature**\n\n");
    markdown.push_str("```destack\n");
    markdown.push_str(signature);
    markdown.push_str("\n```");

    if let Some(documentation) = documentation {
        markdown.push_str("\n\n**Documentation**\n\n");
        markdown.push_str(documentation);
    }

    markdown.push_str("\n\n**Location**\n\n`");
    markdown.push_str(file_path);
    markdown.push(':');
    markdown.push_str(&(line + 1).to_string());
    markdown.push(':');
    markdown.push_str(&(character + 1).to_string());
    markdown.push('`');

    markdown
}

/// Parse parameter labels from one function-like signature string.
fn parameter_labels_from_signature(signature: &str) -> Vec<String> {
    let Some(open_paren) = signature.find('(') else {
        return Vec::new();
    };
    let Some(close_paren) = signature[open_paren + 1..].find(')') else {
        return Vec::new();
    };

    let parameters = &signature[open_paren + 1..open_paren + 1 + close_paren];
    if parameters.trim().is_empty() {
        return Vec::new();
    }

    parameters
        .split(", ")
        .map(|parameter| parameter.to_string())
        .collect()
}

/// Normalize one expected completion kind string into the harness label space.
fn normalize_expected_completion_kind(kind: &str) -> Result<&'static str, String> {
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
