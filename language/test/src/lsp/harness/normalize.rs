use std::path::Path;

use destack_lsp_server::UriExt;
use destack_lsp_types as lsp;

/// One normalized location result for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedLocation {
    /// The target file path inside the virtual workspace.
    pub file_path: String,
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
}

/// One normalized diagnostic result for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedDiagnostic {
    /// The target file path inside the virtual workspace.
    pub file_path: String,
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
    /// The normalized severity label when one exists.
    pub severity: Option<&'static str>,
    /// The normalized diagnostic code when one exists.
    pub code: Option<String>,
    /// The normalized diagnostic source when one exists.
    pub source: Option<String>,
    /// The exact diagnostic message.
    pub message: String,
}

/// One normalized hover result for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedHover {
    /// The normalized hover contents kind.
    pub contents_kind: &'static str,
    /// The normalized hover contents text.
    pub contents: String,
    /// The normalized hover range when one exists.
    pub range: Option<NormalizedLocation>,
}

/// One normalized quick-info result for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedQuickInfo {
    /// The exact signature text.
    pub text: String,
    /// The exact documentation text when one exists.
    pub documentation: Option<String>,
}

/// One normalized signature help result for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedSignatureHelp {
    /// The normalized signatures in response order.
    pub signatures: Vec<NormalizedSignatureInformation>,
    /// The active signature index when one exists.
    pub active_signature: Option<usize>,
    /// The active parameter index when one exists.
    pub active_parameter: Option<usize>,
}

/// One normalized signature information payload.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedSignatureInformation {
    /// The exact signature label.
    pub label: String,
    /// The normalized parameter labels in response order.
    pub parameters: Vec<String>,
}

/// One normalized flattened document symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedDocumentSymbol {
    /// The nesting depth in the document symbol tree.
    pub depth: usize,
    /// The exact symbol name.
    pub name: String,
    /// The normalized symbol kind name.
    pub kind: &'static str,
    /// The enclosing symbol range.
    pub range: NormalizedLocation,
    /// The exact selection range for the symbol name.
    pub selection_range: NormalizedLocation,
}

/// One normalized workspace symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedWorkspaceSymbol {
    /// The exact symbol name.
    pub name: String,
    /// The normalized symbol kind name.
    pub kind: &'static str,
    /// The normalized full symbol location.
    pub location: NormalizedLocation,
    /// The container name when one exists.
    pub container_name: Option<String>,
}

/// One normalized completion item entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedCompletionItem {
    /// The exact completion label.
    pub label: String,
    /// The normalized completion kind name.
    pub kind: &'static str,
}

/// One normalized resolved completion item entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedResolvedCompletionItem {
    /// The exact completion label.
    pub label: String,
    /// The exact resolved documentation when one exists.
    pub documentation: Option<String>,
}

/// One normalized document-link entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedDocumentLink {
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
    /// The normalized target when one exists.
    pub target: Option<String>,
    /// The exact tooltip when one exists.
    pub tooltip: Option<String>,
}

/// One normalized folding-range entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedFoldingRange {
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
    /// The normalized range kind when one exists.
    pub kind: Option<String>,
    /// The collapsed placeholder text when one exists.
    pub collapsed_text: Option<String>,
}

/// One normalized inlay-hint entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedInlayHint {
    /// The zero-based hint line.
    pub line: usize,
    /// The zero-based hint character.
    pub character: usize,
    /// The exact hint label.
    pub label: String,
    /// The normalized hint kind when one exists.
    pub kind: Option<String>,
    /// Whether the hint pads on the left.
    pub padding_left: bool,
    /// Whether the hint pads on the right.
    pub padding_right: bool,
}

/// One normalized code-lens entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedCodeLens {
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
    /// The exact lens title when one exists.
    pub title: Option<String>,
    /// The exact command identifier when one exists.
    pub command: Option<String>,
    /// The exact stringified command arguments.
    pub arguments: Vec<String>,
}

/// One normalized code action entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedCodeAction {
    /// The exact action title.
    pub title: String,
    /// The normalized code action kind name.
    pub kind: String,
    /// Whether the action is preferred.
    pub is_preferred: bool,
    /// Whether the action already carries an edit payload.
    pub has_edit: bool,
}

/// One normalized semantic token entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedSemanticToken {
    /// The token text sliced from the file.
    pub text: String,
    /// The normalized token type name.
    pub token_type: String,
    /// The normalized token modifiers in legend order.
    pub modifiers: Vec<String>,
    /// The zero-based start line.
    pub start_line: usize,
    /// The zero-based start character.
    pub start_character: usize,
    /// The zero-based end line.
    pub end_line: usize,
    /// The zero-based end character.
    pub end_character: usize,
}

/// One normalized hierarchy item for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedHierarchyItem {
    /// The exact item name.
    pub name: String,
    /// The normalized item kind name.
    pub kind: &'static str,
    /// The exact detail payload when one exists.
    pub detail: Option<String>,
    /// The normalized full item range.
    pub range: NormalizedLocation,
    /// The normalized selection range.
    pub selection_range: NormalizedLocation,
}

/// One normalized call hierarchy edge for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedCallHierarchyCall {
    /// The normalized item at the far end of the edge.
    pub item: NormalizedHierarchyItem,
    /// The normalized caller-side call ranges.
    pub call_ranges: Vec<NormalizedLocation>,
}

/// One normalized selection-range chain for exact assertions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NormalizedSelectionRange {
    /// The normalized range chain from the innermost node to the outermost node.
    pub chain: Vec<NormalizedLocation>,
}

/// Normalize one goto definition response into exact comparable locations.
pub fn normalize_definition_response(
    workspace_root: &Path,
    response: &lsp::GotoDefinitionResponse,
) -> Result<Vec<NormalizedLocation>, String> {
    let locations = match response {
        lsp::GotoDefinitionResponse::Scalar(location) => {
            vec![normalize_location(workspace_root, location)?]
        }
        lsp::GotoDefinitionResponse::Array(locations) => locations
            .iter()
            .map(|location| normalize_location(workspace_root, location))
            .collect::<Result<Vec<_>, _>>()?,
        lsp::GotoDefinitionResponse::Link(links) => links
            .iter()
            .map(|link| {
                let location = lsp::Location {
                    uri: link.target_uri.clone(),
                    range: link.target_selection_range,
                };
                normalize_location(workspace_root, &location)
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(sorted_locations(locations))
}

/// Normalize one references response into exact comparable locations.
pub fn normalize_references_response(
    workspace_root: &Path,
    response: &[lsp::Location],
) -> Result<Vec<NormalizedLocation>, String> {
    let locations = response
        .iter()
        .map(|location| normalize_location(workspace_root, location))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sorted_locations(locations))
}

/// Normalize one document highlight response into exact comparable ranges.
pub fn normalize_document_highlights(
    file_path: &str,
    response: &[lsp::DocumentHighlight],
) -> Vec<NormalizedLocation> {
    let locations = response
        .iter()
        .map(|highlight| normalize_range_for_file(file_path, highlight.range))
        .collect::<Vec<_>>();

    sorted_locations(locations)
}

/// Normalize one selection-range response into exact comparable chains.
pub fn normalize_selection_ranges(
    file_path: &str,
    response: &[lsp::SelectionRange],
) -> Vec<NormalizedSelectionRange> {
    let mut ranges = response
        .iter()
        .map(|range| NormalizedSelectionRange {
            chain: normalize_selection_range_chain(file_path, range),
        })
        .collect::<Vec<_>>();

    ranges.sort();

    ranges
}

/// Normalize one prepare rename response into an exact comparable range.
pub fn normalize_prepare_rename(
    file_path: &str,
    response: &lsp::PrepareRenameResponse,
) -> Result<NormalizedLocation, String> {
    let range = match response {
        lsp::PrepareRenameResponse::Range(range) => *range,
        lsp::PrepareRenameResponse::RangeWithPlaceholder { range, .. } => *range,
        lsp::PrepareRenameResponse::DefaultBehavior { default_behavior } => {
            return Err(format!(
                "prepare rename returned default behavior={default_behavior} instead of a concrete range"
            ));
        }
    };

    Ok(normalize_range_for_file(file_path, range))
}

/// Normalize one explicit URI and range into an exact comparable location.
pub fn normalize_uri_range(
    workspace_root: &Path,
    uri: &lsp::Uri,
    range: lsp::Range,
) -> Result<NormalizedLocation, String> {
    normalize_location(
        workspace_root,
        &lsp::Location {
            uri: uri.clone(),
            range,
        },
    )
}

/// Normalize one LSP location against the materialized workspace root.
pub fn normalize_location(
    workspace_root: &Path,
    location: &lsp::Location,
) -> Result<NormalizedLocation, String> {
    let path = location
        .uri
        .to_file_path()
        .ok_or_else(|| format!("location uri is not a file path: {:?}", location.uri))?;
    let relative_path = path
        .strip_prefix(workspace_root)
        .map_err(|_| {
            format!(
                "location path {} is outside workspace root {}",
                path.display(),
                workspace_root.display()
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");

    Ok(NormalizedLocation {
        file_path: relative_path,
        start_line: location.range.start.line as usize,
        start_character: location.range.start.character as usize,
        end_line: location.range.end.line as usize,
        end_character: location.range.end.character as usize,
    })
}

/// Normalize one plain LSP range for a known fixture-relative file path.
pub fn normalize_range_for_file(file_path: &str, range: lsp::Range) -> NormalizedLocation {
    NormalizedLocation {
        file_path: file_path.to_string(),
        start_line: range.start.line as usize,
        start_character: range.start.character as usize,
        end_line: range.end.line as usize,
        end_character: range.end.character as usize,
    }
}

/// Return the number of text edits contained in one workspace edit.
pub fn workspace_edit_edit_count(edit: &lsp::WorkspaceEdit) -> usize {
    let document_change_count = edit
        .document_changes
        .as_ref()
        .map(|changes| match changes {
            lsp::DocumentChanges::Edits(edits) => edits.iter().map(|edit| edit.edits.len()).sum(),
            lsp::DocumentChanges::Operations(changes) => changes
                .iter()
                .map(|change| match change {
                    lsp::DocumentChangeOperation::Op(_) => 0,
                    lsp::DocumentChangeOperation::Edit(edit) => edit.edits.len(),
                })
                .sum::<usize>(),
        })
        .unwrap_or(0);
    let change_count = edit
        .changes
        .as_ref()
        .map(|changes| changes.values().map(std::vec::Vec::len).sum::<usize>())
        .unwrap_or(0);

    document_change_count + change_count
}

/// Return one stably ordered location list for exact comparisons.
pub fn sorted_locations(mut locations: Vec<NormalizedLocation>) -> Vec<NormalizedLocation> {
    locations.sort();

    locations
}

/// Normalize one diagnostic list for a fixture-relative file path.
pub fn normalize_diagnostics(
    file_path: &str,
    diagnostics: &[lsp::Diagnostic],
) -> Vec<NormalizedDiagnostic> {
    let diagnostics = diagnostics
        .iter()
        .map(|diagnostic| NormalizedDiagnostic {
            file_path: file_path.to_string(),
            start_line: diagnostic.range.start.line as usize,
            start_character: diagnostic.range.start.character as usize,
            end_line: diagnostic.range.end.line as usize,
            end_character: diagnostic.range.end.character as usize,
            severity: normalize_diagnostic_severity(diagnostic.severity),
            code: normalize_diagnostic_code(diagnostic.code.as_ref()),
            source: diagnostic.source.clone(),
            message: diagnostic.message.clone(),
        })
        .collect::<Vec<_>>();

    sorted_diagnostics(diagnostics)
}

/// Normalize one full workspace diagnostic report into an exact diagnostic list.
pub fn normalize_workspace_diagnostic_report(
    workspace_root: &Path,
    report: &lsp::WorkspaceDiagnosticReportResult,
) -> Result<Vec<NormalizedDiagnostic>, String> {
    let lsp::WorkspaceDiagnosticReportResult::Report(report) = report else {
        return Err("expected full workspace diagnostic report".to_string());
    };

    normalize_workspace_document_reports(workspace_root, &report.items)
}

/// Normalize one partial workspace diagnostic report into an exact diagnostic list.
pub fn normalize_workspace_diagnostic_partial_report(
    workspace_root: &Path,
    report: &lsp::WorkspaceDiagnosticReportPartialResult,
) -> Result<Vec<NormalizedDiagnostic>, String> {
    normalize_workspace_document_reports(workspace_root, &report.items)
}

/// Normalize one workspace diagnostic document report list into exact diagnostics.
fn normalize_workspace_document_reports(
    workspace_root: &Path,
    reports: &[lsp::WorkspaceDocumentDiagnosticReport],
) -> Result<Vec<NormalizedDiagnostic>, String> {
    let mut diagnostics = Vec::new();

    // flatten each full document report into one stable diagnostic list
    for report in reports {
        match report {
            lsp::WorkspaceDocumentDiagnosticReport::Full(full) => {
                let path = full.uri.to_file_path().ok_or_else(|| {
                    format!(
                        "workspace diagnostic uri is not a file path: {:?}",
                        full.uri
                    )
                })?;
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

                diagnostics.extend(normalize_diagnostics(
                    &relative_path,
                    &full.full_document_diagnostic_report.items,
                ));
            }

            // previous-result based incremental reports are not expected in canonical fixtures yet
            lsp::WorkspaceDocumentDiagnosticReport::Unchanged(_) => {
                return Err("unexpected unchanged workspace diagnostic report".to_string());
            }
        }
    }

    Ok(sorted_diagnostics(diagnostics))
}

/// Normalize one selection range chain from inner to outer ranges.
fn normalize_selection_range_chain(
    file_path: &str,
    range: &lsp::SelectionRange,
) -> Vec<NormalizedLocation> {
    let mut chain = Vec::new();
    let mut current = Some(range);

    // flatten parent links so assertions can compare full expansion order
    while let Some(selection_range) = current {
        chain.push(normalize_range_for_file(file_path, selection_range.range));
        current = selection_range.parent.as_deref();
    }

    chain
}

/// Normalize one hover response into an exact comparable shape.
pub fn normalize_hover(
    workspace_root: &Path,
    file_path: &str,
    hover: &lsp::Hover,
) -> Result<NormalizedHover, String> {
    let range = hover
        .range
        .map(|range| {
            normalize_location(
                workspace_root,
                &lsp::Location {
                    uri: lsp::Uri::from_file_path(workspace_root.join(file_path)).ok_or_else(
                        || {
                            format!(
                                "failed to build file uri for normalized hover path {file_path}"
                            )
                        },
                    )?,
                    range,
                },
            )
        })
        .transpose()?;

    let (contents_kind, contents) = normalize_hover_contents(workspace_root, &hover.contents);

    Ok(NormalizedHover {
        contents_kind,
        contents,
        range,
    })
}

/// Normalize one hover payload into a tsserver-like quick-info shape.
pub fn normalize_quick_info(
    workspace_root: &Path,
    file_path: &str,
    hover: &lsp::Hover,
) -> Result<NormalizedQuickInfo, String> {
    let normalized_hover = normalize_hover(workspace_root, file_path, hover)?;
    let contents = normalized_hover.contents;

    let signature_prefix = "**Signature**\n\n```destack\n";
    let documentation_prefix = "\n```\n\n**Documentation**\n\n";
    let location_prefix = "\n\n**Location**\n\n`";
    let signature_tail = contents
        .strip_prefix(signature_prefix)
        .ok_or_else(|| format!("quick info is missing signature prefix\ncontents: {contents}"))?;

    // split the normalized markdown into the signature body and optional documentation
    if let Some((signature, tail)) = signature_tail.split_once(documentation_prefix) {
        let (documentation, _) = tail.split_once(location_prefix).ok_or_else(|| {
            format!(
                "quick info with documentation is missing location suffix\ncontents: {contents}"
            )
        })?;

        return Ok(NormalizedQuickInfo {
            text: signature.to_string(),
            documentation: Some(documentation.to_string()),
        });
    }

    let (signature, _) = signature_tail
        .split_once(location_prefix)
        .ok_or_else(|| format!("quick info is missing location suffix\ncontents: {contents}"))?;

    Ok(NormalizedQuickInfo {
        text: signature
            .strip_suffix("\n```")
            .unwrap_or(signature)
            .to_string(),
        documentation: None,
    })
}

/// Normalize one signature help response into an exact comparable shape.
pub fn normalize_signature_help(help: &lsp::SignatureHelp) -> NormalizedSignatureHelp {
    let signatures = help
        .signatures
        .iter()
        .map(|signature| NormalizedSignatureInformation {
            label: signature.label.clone(),
            parameters: signature
                .parameters
                .as_ref()
                .into_iter()
                .flatten()
                .map(normalize_parameter_label)
                .collect(),
        })
        .collect();

    NormalizedSignatureHelp {
        signatures,
        active_signature: help.active_signature.map(|index| index as usize),
        active_parameter: help.active_parameter.map(|index| index as usize),
    }
}

/// Normalize one document symbol response into a flattened exact comparable list.
pub fn normalize_document_symbols(
    workspace_root: &Path,
    file_path: &str,
    response: &lsp::DocumentSymbolResponse,
) -> Result<Vec<NormalizedDocumentSymbol>, String> {
    let mut normalized = Vec::new();

    // flatten the document symbol response into one exact ordered list
    match response {
        lsp::DocumentSymbolResponse::Nested(symbols) => {
            for symbol in symbols {
                flatten_document_symbol(workspace_root, file_path, symbol, 0, &mut normalized)?;
            }
        }
        lsp::DocumentSymbolResponse::Flat(symbols) => {
            for symbol in symbols {
                let location = normalize_location(workspace_root, &symbol.location)?;
                normalized.push(NormalizedDocumentSymbol {
                    depth: 0,
                    name: symbol.name.clone(),
                    kind: normalize_symbol_kind(symbol.kind)?,
                    range: location.clone(),
                    selection_range: location,
                });
            }
        }
    }

    Ok(normalized)
}

/// Normalize one workspace symbol response into an exact comparable list.
pub fn normalize_workspace_symbols(
    workspace_root: &Path,
    response: &lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>,
) -> Result<Vec<NormalizedWorkspaceSymbol>, String> {
    // normalize both protocol variants into one stable symbol list
    match response {
        lsp::OneOf::Left(symbols) => symbols
            .iter()
            .map(|symbol| {
                Ok(NormalizedWorkspaceSymbol {
                    name: symbol.name.clone(),
                    kind: normalize_symbol_kind(symbol.kind)?,
                    location: normalize_location(workspace_root, &symbol.location)?,
                    container_name: symbol.container_name.clone(),
                })
            })
            .collect(),
        lsp::OneOf::Right(symbols) => symbols
            .iter()
            .map(|symbol| {
                let lsp::OneOf::Left(location) = &symbol.location else {
                    return Err(format!(
                        "workspace symbol {} did not include a full location range",
                        symbol.name
                    ));
                };

                Ok(NormalizedWorkspaceSymbol {
                    name: symbol.name.clone(),
                    kind: normalize_symbol_kind(symbol.kind)?,
                    location: normalize_location(workspace_root, location)?,
                    container_name: symbol.container_name.clone(),
                })
            })
            .collect(),
    }
}

/// Normalize one call hierarchy item into an exact comparable shape.
pub fn normalize_call_hierarchy_item(
    workspace_root: &Path,
    item: &lsp::CallHierarchyItem,
) -> Result<NormalizedHierarchyItem, String> {
    let range = normalize_uri_range(workspace_root, &item.uri, item.range)?;
    let selection_range = normalize_uri_range(workspace_root, &item.uri, item.selection_range)?;

    Ok(NormalizedHierarchyItem {
        name: item.name.clone(),
        kind: normalize_symbol_kind(item.kind)?,
        detail: item.detail.clone(),
        range,
        selection_range,
    })
}

/// Normalize incoming call hierarchy edges into an exact comparable shape.
pub fn normalize_call_hierarchy_incoming(
    workspace_root: &Path,
    calls: &[lsp::CallHierarchyIncomingCall],
) -> Result<Vec<NormalizedCallHierarchyCall>, String> {
    let mut normalized = calls
        .iter()
        .map(|call| {
            let item = normalize_call_hierarchy_item(workspace_root, &call.from)?;
            let call_ranges = call
                .from_ranges
                .iter()
                .copied()
                .map(|range| normalize_uri_range(workspace_root, &call.from.uri, range))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(NormalizedCallHierarchyCall { item, call_ranges })
        })
        .collect::<Result<Vec<_>, String>>()?;
    normalized.sort();

    Ok(normalized)
}

/// Normalize outgoing call hierarchy edges into an exact comparable shape.
pub fn normalize_call_hierarchy_outgoing(
    workspace_root: &Path,
    calls: &[lsp::CallHierarchyOutgoingCall],
) -> Result<Vec<NormalizedCallHierarchyCall>, String> {
    let mut normalized = calls
        .iter()
        .map(|call| {
            let item = normalize_call_hierarchy_item(workspace_root, &call.to)?;
            let call_ranges = call
                .from_ranges
                .iter()
                .copied()
                .map(|range| normalize_uri_range(workspace_root, &call.to.uri, range))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(NormalizedCallHierarchyCall { item, call_ranges })
        })
        .collect::<Result<Vec<_>, String>>()?;
    normalized.sort();

    Ok(normalized)
}

/// Normalize type hierarchy items into an exact comparable shape.
pub fn normalize_type_hierarchy_items(
    workspace_root: &Path,
    items: &[lsp::TypeHierarchyItem],
) -> Result<Vec<NormalizedHierarchyItem>, String> {
    let mut normalized = items
        .iter()
        .map(|item| {
            let range = normalize_uri_range(workspace_root, &item.uri, item.range)?;
            let selection_range =
                normalize_uri_range(workspace_root, &item.uri, item.selection_range)?;

            Ok(NormalizedHierarchyItem {
                name: item.name.clone(),
                kind: normalize_symbol_kind(item.kind)?,
                detail: item.detail.clone(),
                range,
                selection_range,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    normalized.sort();

    Ok(normalized)
}

/// Normalize one completion response into an exact ordered item list.
pub fn normalize_completion_response(
    response: &lsp::CompletionResponse,
) -> Result<Vec<NormalizedCompletionItem>, String> {
    let items = match response {
        lsp::CompletionResponse::Array(items) => items,
        lsp::CompletionResponse::List(list) => &list.items,
    };

    items
        .iter()
        .map(|item| {
            let kind = item
                .kind
                .ok_or_else(|| format!("completion item {} is missing a kind", item.label))?;

            Ok(NormalizedCompletionItem {
                label: item.label.clone(),
                kind: normalize_completion_kind(kind)?,
            })
        })
        .collect()
}

/// Normalize one resolved completion item into an exact comparable shape.
pub fn normalize_resolved_completion_item(
    item: &lsp::CompletionItem,
) -> Result<NormalizedResolvedCompletionItem, String> {
    let documentation = item
        .documentation
        .as_ref()
        .map(normalize_completion_documentation)
        .transpose()?;

    Ok(NormalizedResolvedCompletionItem {
        label: item.label.clone(),
        documentation,
    })
}

/// Normalize one document-link response into exact comparable links.
pub fn normalize_document_links(
    workspace_root: &Path,
    response: &[lsp::DocumentLink],
) -> Result<Vec<NormalizedDocumentLink>, String> {
    let mut links = response
        .iter()
        .map(|link| {
            Ok(NormalizedDocumentLink {
                start_line: link.range.start.line as usize,
                start_character: link.range.start.character as usize,
                end_line: link.range.end.line as usize,
                end_character: link.range.end.character as usize,
                target: link
                    .target
                    .as_ref()
                    .map(|target| normalize_optional_target_uri(workspace_root, target))
                    .transpose()?,
                tooltip: link.tooltip.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    links.sort();

    Ok(links)
}

/// Normalize one folding-range response into exact comparable ranges.
pub fn normalize_folding_ranges(response: &[lsp::FoldingRange]) -> Vec<NormalizedFoldingRange> {
    let mut ranges = response
        .iter()
        .map(|range| NormalizedFoldingRange {
            start_line: range.start_line as usize,
            start_character: range.start_character.unwrap_or(0) as usize,
            end_line: range.end_line as usize,
            end_character: range.end_character.unwrap_or(0) as usize,
            kind: range.kind.as_ref().map(normalize_folding_range_kind),
            collapsed_text: range.collapsed_text.clone(),
        })
        .collect::<Vec<_>>();
    ranges.sort();

    ranges
}

/// Normalize one inlay-hint response into exact comparable hints.
pub fn normalize_inlay_hints(
    response: &[lsp::InlayHint],
) -> Result<Vec<NormalizedInlayHint>, String> {
    let mut hints = response
        .iter()
        .map(|hint| {
            Ok(NormalizedInlayHint {
                line: hint.position.line as usize,
                character: hint.position.character as usize,
                label: normalize_inlay_hint_label(&hint.label)?,
                kind: hint.kind.as_ref().map(normalize_inlay_hint_kind),
                padding_left: hint.padding_left.unwrap_or(false),
                padding_right: hint.padding_right.unwrap_or(false),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    hints.sort();

    Ok(hints)
}

/// Normalize one code-lens response into exact comparable lenses.
pub fn normalize_code_lenses(response: &[lsp::CodeLens]) -> Vec<NormalizedCodeLens> {
    let mut lenses = response
        .iter()
        .map(|lens| NormalizedCodeLens {
            start_line: lens.range.start.line as usize,
            start_character: lens.range.start.character as usize,
            end_line: lens.range.end.line as usize,
            end_character: lens.range.end.character as usize,
            title: lens.command.as_ref().map(|command| command.title.clone()),
            command: lens.command.as_ref().map(|command| command.command.clone()),
            arguments: lens
                .command
                .as_ref()
                .and_then(|command| command.arguments.as_ref())
                .map(|arguments| {
                    arguments
                        .iter()
                        .map(normalize_lsp_any_argument)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    lenses.sort();

    lenses
}

/// Normalize one code action response into an exact comparable list.
pub fn normalize_code_actions(
    response: &[lsp::CodeActionOrCommand],
) -> Result<Vec<NormalizedCodeAction>, String> {
    response
        .iter()
        .map(|action| {
            let lsp::CodeActionOrCommand::CodeAction(action) = action else {
                return Err("expected code action entries, not bare commands".to_string());
            };

            Ok(NormalizedCodeAction {
                title: action.title.clone(),
                kind: action
                    .kind
                    .as_ref()
                    .map(|kind| normalize_code_action_kind(kind.as_str()))
                    .unwrap_or_default(),
                is_preferred: action.is_preferred.unwrap_or(false),
                has_edit: action.edit.is_some(),
            })
        })
        .collect()
}

/// Normalize one semantic tokens result into exact file-relative tokens.
pub fn normalize_semantic_tokens(
    text: &str,
    result: &lsp::SemanticTokensResult,
) -> Result<Vec<NormalizedSemanticToken>, String> {
    let tokens = match result {
        lsp::SemanticTokensResult::Tokens(tokens) => &tokens.data,
        lsp::SemanticTokensResult::Partial(partial) => &partial.data,
    };

    let mut absolute_line = 0u32;
    let mut absolute_character = 0u32;
    let mut normalized = Vec::new();
    for token in tokens {
        if token.delta_line == 0 {
            absolute_character += token.delta_start;
        } else {
            absolute_line += token.delta_line;
            absolute_character = token.delta_start;
        }

        let start_offset = offset_for_line_character(text, absolute_line, absolute_character)?;
        let end_offset =
            offset_for_line_character(text, absolute_line, absolute_character + token.length)?;
        let token_text = text
            .get(start_offset..end_offset)
            .ok_or_else(|| "semantic token range does not align to char boundaries".to_string())?;

        normalized.push(NormalizedSemanticToken {
            text: token_text.to_string(),
            token_type: semantic_token_type_name(token.token_type)?.to_string(),
            modifiers: semantic_token_modifier_names(token.token_modifiers_bitset)?,
            start_line: absolute_line as usize,
            start_character: absolute_character as usize,
            end_line: absolute_line as usize,
            end_character: (absolute_character + token.length) as usize,
        });
    }

    Ok(normalized)
}

/// Return one stably ordered diagnostics list for exact comparisons.
pub fn sorted_diagnostics(mut diagnostics: Vec<NormalizedDiagnostic>) -> Vec<NormalizedDiagnostic> {
    diagnostics.sort();

    diagnostics
}

/// Normalize one LSP diagnostic severity into a stable label.
fn normalize_diagnostic_severity(
    severity: Option<lsp::DiagnosticSeverity>,
) -> Option<&'static str> {
    match severity {
        Some(lsp::DiagnosticSeverity::ERROR) => Some("error"),
        Some(lsp::DiagnosticSeverity::WARNING) => Some("warning"),
        Some(lsp::DiagnosticSeverity::INFORMATION) => Some("information"),
        Some(lsp::DiagnosticSeverity::HINT) => Some("hint"),
        Some(_) => Some("unknown"),
        None => None,
    }
}

/// Normalize one LSP diagnostic code into a stable string.
fn normalize_diagnostic_code(code: Option<&lsp::NumberOrString>) -> Option<String> {
    match code {
        Some(lsp::NumberOrString::Number(code)) => Some(code.to_string()),
        Some(lsp::NumberOrString::String(code)) => Some(code.clone()),
        None => None,
    }
}

/// Normalize one hover contents payload into a stable kind and body.
fn normalize_hover_contents(
    workspace_root: &Path,
    contents: &lsp::HoverContents,
) -> (&'static str, String) {
    match contents {
        lsp::HoverContents::Markup(markup) => (
            match markup.kind {
                lsp::MarkupKind::Markdown => "markdown",
                lsp::MarkupKind::PlainText => "plaintext",
            },
            normalize_hover_text(workspace_root, &markup.value),
        ),
        lsp::HoverContents::Scalar(scalar) => ("scalar", normalize_hover_marked_string(scalar)),
        lsp::HoverContents::Array(items) => (
            "array",
            items
                .iter()
                .map(normalize_hover_marked_string)
                .collect::<Vec<_>>()
                .join("\n---\n"),
        ),
    }
}

/// Normalize one completion documentation payload.
fn normalize_completion_documentation(
    documentation: &lsp::Documentation,
) -> Result<String, String> {
    match documentation {
        lsp::Documentation::String(text) => Ok(text.clone()),
        lsp::Documentation::MarkupContent(markup) => Ok(markup.value.clone()),
    }
}

/// Normalize one optional target URI into a stable relative or literal string.
fn normalize_optional_target_uri(
    workspace_root: &Path,
    target: &lsp::Uri,
) -> Result<String, String> {
    if let Some(path) = target.to_file_path() {
        let relative_path = path
            .strip_prefix(workspace_root)
            .map_err(|_| {
                format!(
                    "document-link target path {} is outside workspace root {}",
                    path.display(),
                    workspace_root.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");

        let target_text = if let Some(fragment) = target.fragment() {
            format!("{relative_path}#{fragment}")
        } else {
            relative_path
        };

        return Ok(target_text);
    }

    Ok(target.as_str().to_string())
}

/// Normalize one inlay-hint label payload into exact text.
fn normalize_inlay_hint_label(label: &lsp::InlayHintLabel) -> Result<String, String> {
    match label {
        lsp::InlayHintLabel::String(text) => Ok(text.clone()),
        lsp::InlayHintLabel::LabelParts(parts) => {
            let mut text = String::new();

            // flatten label parts in response order
            for part in parts {
                text.push_str(&part.value);
            }

            Ok(text)
        }
    }
}

/// Normalize one inlay-hint kind into a stable label.
fn normalize_inlay_hint_kind(kind: &lsp::InlayHintKind) -> String {
    match *kind {
        lsp::InlayHintKind::TYPE => "type".to_string(),
        lsp::InlayHintKind::PARAMETER => "parameter".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Normalize one folding-range kind into a stable label.
fn normalize_folding_range_kind(kind: &lsp::FoldingRangeKind) -> String {
    match kind {
        lsp::FoldingRangeKind::Comment => "comment".to_string(),
        lsp::FoldingRangeKind::Imports => "imports".to_string(),
        lsp::FoldingRangeKind::Region => "region".to_string(),
    }
}

/// Normalize one LSP argument into a stable exact string.
fn normalize_lsp_any_argument(value: &lsp::LSPAny) -> String {
    match value {
        lsp::LSPAny::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

/// Flatten one nested document symbol tree into depth-annotated entries.
fn flatten_document_symbol(
    workspace_root: &Path,
    file_path: &str,
    symbol: &lsp::DocumentSymbol,
    depth: usize,
    normalized: &mut Vec<NormalizedDocumentSymbol>,
) -> Result<(), String> {
    let range = normalize_range(workspace_root, file_path, symbol.range)?;
    let selection_range = normalize_range(workspace_root, file_path, symbol.selection_range)?;

    normalized.push(NormalizedDocumentSymbol {
        depth,
        name: symbol.name.clone(),
        kind: normalize_symbol_kind(symbol.kind)?,
        range: range.clone(),
        selection_range,
    });

    // recurse into children in response order
    if let Some(children) = &symbol.children {
        for child in children {
            flatten_document_symbol(workspace_root, file_path, child, depth + 1, normalized)?;
        }
    }

    Ok(())
}

/// Normalize one range into a fixture-relative location.
fn normalize_range(
    workspace_root: &Path,
    file_path: &str,
    range: lsp::Range,
) -> Result<NormalizedLocation, String> {
    let file_path = if file_path.is_empty() {
        return Err("normalize_range requires a file path".to_string());
    } else {
        file_path
    };
    let uri = lsp::Uri::from_file_path(workspace_root.join(file_path))
        .ok_or_else(|| format!("failed to build file uri for normalized range path {file_path}"))?;

    normalize_location(workspace_root, &lsp::Location { uri, range })
}

/// Normalize one symbol kind into a stable lowercase label.
fn normalize_symbol_kind(kind: lsp::SymbolKind) -> Result<&'static str, String> {
    match kind {
        lsp::SymbolKind::FILE => Ok("file"),
        lsp::SymbolKind::MODULE => Ok("module"),
        lsp::SymbolKind::NAMESPACE => Ok("namespace"),
        lsp::SymbolKind::PACKAGE => Ok("package"),
        lsp::SymbolKind::CLASS => Ok("class"),
        lsp::SymbolKind::METHOD => Ok("method"),
        lsp::SymbolKind::PROPERTY => Ok("property"),
        lsp::SymbolKind::FIELD => Ok("field"),
        lsp::SymbolKind::CONSTRUCTOR => Ok("constructor"),
        lsp::SymbolKind::ENUM => Ok("enum"),
        lsp::SymbolKind::INTERFACE => Ok("interface"),
        lsp::SymbolKind::FUNCTION => Ok("function"),
        lsp::SymbolKind::VARIABLE => Ok("variable"),
        lsp::SymbolKind::CONSTANT => Ok("constant"),
        lsp::SymbolKind::STRING => Ok("string"),
        lsp::SymbolKind::NUMBER => Ok("number"),
        lsp::SymbolKind::BOOLEAN => Ok("boolean"),
        lsp::SymbolKind::ARRAY => Ok("array"),
        lsp::SymbolKind::OBJECT => Ok("object"),
        lsp::SymbolKind::KEY => Ok("key"),
        lsp::SymbolKind::NULL => Ok("null"),
        lsp::SymbolKind::ENUM_MEMBER => Ok("enum_member"),
        lsp::SymbolKind::STRUCT => Ok("struct"),
        lsp::SymbolKind::EVENT => Ok("event"),
        lsp::SymbolKind::OPERATOR => Ok("operator"),
        lsp::SymbolKind::TYPE_PARAMETER => Ok("type_parameter"),
        _ => Err(format!("unsupported symbol kind value {kind:?}")),
    }
}

/// Normalize one completion item kind into a stable lowercase label.
fn normalize_completion_kind(kind: lsp::CompletionItemKind) -> Result<&'static str, String> {
    match kind {
        lsp::CompletionItemKind::TEXT => Ok("text"),
        lsp::CompletionItemKind::METHOD => Ok("method"),
        lsp::CompletionItemKind::FUNCTION => Ok("function"),
        lsp::CompletionItemKind::CONSTRUCTOR => Ok("constructor"),
        lsp::CompletionItemKind::FIELD => Ok("field"),
        lsp::CompletionItemKind::VARIABLE => Ok("variable"),
        lsp::CompletionItemKind::CLASS => Ok("class"),
        lsp::CompletionItemKind::INTERFACE => Ok("interface"),
        lsp::CompletionItemKind::MODULE => Ok("module"),
        lsp::CompletionItemKind::PROPERTY => Ok("property"),
        lsp::CompletionItemKind::UNIT => Ok("unit"),
        lsp::CompletionItemKind::VALUE => Ok("value"),
        lsp::CompletionItemKind::ENUM => Ok("enum"),
        lsp::CompletionItemKind::KEYWORD => Ok("keyword"),
        lsp::CompletionItemKind::SNIPPET => Ok("snippet"),
        lsp::CompletionItemKind::COLOR => Ok("color"),
        lsp::CompletionItemKind::FILE => Ok("file"),
        lsp::CompletionItemKind::REFERENCE => Ok("reference"),
        lsp::CompletionItemKind::FOLDER => Ok("folder"),
        lsp::CompletionItemKind::ENUM_MEMBER => Ok("enum_member"),
        lsp::CompletionItemKind::CONSTANT => Ok("constant"),
        lsp::CompletionItemKind::STRUCT => Ok("struct"),
        lsp::CompletionItemKind::EVENT => Ok("event"),
        lsp::CompletionItemKind::OPERATOR => Ok("operator"),
        lsp::CompletionItemKind::TYPE_PARAMETER => Ok("type_parameter"),
        _ => Err(format!("unsupported completion kind value {kind:?}")),
    }
}

/// Normalize one code action kind string into a stable snake-case label.
fn normalize_code_action_kind(kind: &str) -> String {
    match kind {
        "quickfix" => "quick_fix".to_string(),
        "refactor" => "refactor".to_string(),
        "refactor.extract" => "refactor_extract".to_string(),
        "refactor.inline" => "refactor_inline".to_string(),
        "refactor.rewrite" => "refactor_rewrite".to_string(),
        "source" => "source".to_string(),
        "source.organizeImports" => "source_organize_imports".to_string(),
        "source.fixAll" => "source_fix_all".to_string(),
        other => other.replace('.', "_"),
    }
}

/// Return one semantic token type name from the legend index.
fn semantic_token_type_name(index: u32) -> Result<&'static str, String> {
    match index {
        0 => Ok("namespace"),
        1 => Ok("type"),
        2 => Ok("class"),
        3 => Ok("enum"),
        4 => Ok("interface"),
        5 => Ok("struct"),
        6 => Ok("type_parameter"),
        7 => Ok("parameter"),
        8 => Ok("variable"),
        9 => Ok("property"),
        10 => Ok("enum_member"),
        11 => Ok("function"),
        12 => Ok("method"),
        13 => Ok("macro"),
        14 => Ok("keyword"),
        15 => Ok("modifier"),
        16 => Ok("comment"),
        17 => Ok("string"),
        18 => Ok("number"),
        19 => Ok("regexp"),
        20 => Ok("operator"),
        21 => Ok("decorator"),
        _ => Err(format!(
            "semantic token type index {index} is out of bounds"
        )),
    }
}

/// Return semantic token modifier names from the legend bitset.
fn semantic_token_modifier_names(bitset: u32) -> Result<Vec<String>, String> {
    let mut names = Vec::new();

    for bit in 0..32 {
        if bitset & (1 << bit) == 0 {
            continue;
        }

        let modifier = match bit {
            0 => "declaration",
            1 => "definition",
            2 => "readonly",
            3 => "static",
            4 => "deprecated",
            5 => "abstract",
            6 => "async",
            7 => "modification",
            8 => "documentation",
            9 => "defaultLibrary",
            10 => "mutable",
            _ => {
                return Err(format!(
                    "semantic token modifier bit {bit} is out of bounds"
                ));
            }
        };
        names.push(modifier.to_string());
    }

    Ok(names)
}

/// Convert one zero-based UTF-16 line and character into a byte offset.
fn offset_for_line_character(
    text: &str,
    target_line: u32,
    target_character: u32,
) -> Result<usize, String> {
    let target_line = target_line as usize;
    let target_character = target_character as usize;
    let mut line = 0usize;
    let mut character = 0usize;

    for (offset, ch) in text.char_indices() {
        if line == target_line && character == target_character {
            return Ok(offset);
        }

        if ch == '\n' {
            line += 1;
            character = 0;
            continue;
        }

        character += ch.len_utf16();
    }

    if line == target_line && character == target_character {
        return Ok(text.len());
    }

    Err(format!(
        "semantic token position {}:{} is outside document bounds",
        target_line, target_character
    ))
}

/// Normalize one hover marked string into a stable text form.
fn normalize_hover_marked_string(value: &lsp::MarkedString) -> String {
    match value {
        lsp::MarkedString::String(text) => text.clone(),
        lsp::MarkedString::LanguageString(string) => {
            format!("```{}\n{}\n```", string.language, string.value)
        }
    }
}

/// Strip workspace-absolute path prefixes from hover markdown.
fn normalize_hover_text(workspace_root: &Path, text: &str) -> String {
    let workspace_root = workspace_root.to_string_lossy().replace('\\', "/");
    let workspace_prefix = format!("{workspace_root}/");

    text.replace(&workspace_prefix, "")
}

/// Normalize one parameter label into a stable string.
fn normalize_parameter_label(parameter: &lsp::ParameterInformation) -> String {
    match &parameter.label {
        lsp::ParameterLabel::Simple(label) => label.clone(),
        lsp::ParameterLabel::LabelOffsets([start, end]) => format!("{start}:{end}"),
    }
}
