use std::collections::HashSet;
use std::fmt::{self, Debug, Display, Formatter};

use tspp_dir::DecoratorTarget;
use tspp_query::{
    CallItem, CodeAction, CodeLens, CodeLensAction, CompletionDetailsResponse,
    CompletionEntryDetails, CompletionResponse, DecoratorItem, FoldingRange, Hover, IncomingCall,
    InlayHint, Link, NavigationTarget, OutgoingCall, OutlineSymbol, QueryResponse, SearchSymbol,
    SelectionRange, SemanticToken, SemanticTokenModifiers, SignatureHelp, Target, TypeItem,
};
use tspp_source::{DiagnosticTarget, Patch, PatchSet};

use super::{QueryCall, QueryRun, parse_words};

/// One query response rendered as exact ordered rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResponseRows {
    /// The response rows in query order.
    rows: Vec<QueryRow>,
}

impl ResponseRows {
    /// Parse expected response rows.
    pub(super) fn parse(source: &str, method: &str) -> Result<Self, String> {
        let mut rows = Vec::new();

        // parse every non-empty response line
        for line in source.lines().filter(|line| !line.trim().is_empty()) {
            rows.push(QueryRow::parse(line, method)?);
        }
        if rows.is_empty() {
            return Err("query response has no rows".to_string());
        }
        let response = Self { rows };
        let canonical = format!("{response}\n");
        if source != canonical {
            return Err(format!(
                "query response rows are not canonical\nexpected:\n{canonical}actual:\n{source}"
            ));
        }

        Ok(response)
    }

    /// Create response rows.
    fn new(rows: Vec<QueryRow>) -> Self {
        Self { rows }
    }
}

impl Display for ResponseRows {
    /// Render canonical response rows.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for (index, row) in self.rows.iter().enumerate() {
            if index > 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{row}")?;
        }

        Ok(())
    }
}

/// One exact response entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryRow {
    /// The row noun.
    noun: String,
    /// The ordered row fields.
    fields: Vec<QueryField>,
}

impl QueryRow {
    /// Create one response row.
    fn new(noun: impl Into<String>) -> Self {
        Self {
            noun: noun.into(),
            fields: Vec::new(),
        }
    }

    /// Parse one expected response row.
    fn parse(source: &str, method: &str) -> Result<Self, String> {
        let words = parse_words(source)?;
        let Some(noun) = words.first() else {
            return Err("query response row is empty".to_string());
        };
        let Some(noun) = noun.strip_prefix('@') else {
            return Err(format!("query response row '{source}' must start with '@'"));
        };

        // require the exact fixture method and one response entry
        let Some((row_method, entry)) = noun.split_once('.') else {
            return Err(format!(
                "query response noun '{noun}' must be '<method>.<entry>'"
            ));
        };
        if row_method.is_empty() || entry.is_empty() {
            return Err(format!(
                "query response noun '{noun}' must be '<method>.<entry>'"
            ));
        } else if row_method != method {
            return Err(format!(
                "query response noun '{noun}' must belong to method '{method}'"
            ));
        }
        let mut row = Self::new(noun);

        // parse every exact ordered field
        for word in &words[1..] {
            let Some((name, value)) = word.split_once('=') else {
                return Err(format!(
                    "query response field '{word}' must be '<name>=<value>'"
                ));
            };
            if name.is_empty() {
                return Err(format!(
                    "query response field '{word}' must be '<name>=<value>'"
                ));
            }
            if row.fields.iter().any(|field| field.name == name) {
                return Err(format!("query response row repeats field '{name}'"));
            }
            row.fields.push(QueryField {
                name: name.to_string(),
                value: value.to_string(),
            });
        }

        Ok(row)
    }

    /// Append one required field.
    fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(QueryField {
            name: name.into(),
            value: value.into(),
        });

        self
    }

    /// Append one optional field when present.
    fn optional(self, name: &str, value: Option<impl Into<String>>) -> Self {
        match value {
            Some(value) => self.field(name, value),
            None => self,
        }
    }

    /// Append one true boolean while omitting the false default.
    fn flag(self, name: &str, value: bool) -> Self {
        if value {
            self.field(name, "true")
        } else {
            self
        }
    }

    /// Append one exact source target.
    fn with_target(self, run: &QueryRun<'_>, target: &Target) -> Result<Self, String> {
        // format the full source range
        run.require_module(target.module, target.span.file)?;
        let location = run.format_span(target.span)?;

        // omit a duplicate primary selection
        let selection = if target.selection_span == target.span {
            None
        } else {
            Some(run.format_span(target.selection_span)?)
        };

        Ok(self
            .field("location", location)
            .optional("selection", selection))
    }
}

impl Display for QueryRow {
    /// Render one canonical response row.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "@{}", self.noun)?;

        // render every field in its declared order
        for field in &self.fields {
            write!(formatter, " {field}")?;
        }

        Ok(())
    }
}

/// One named response row value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryField {
    /// The field name.
    name: String,
    /// The exact field value.
    value: String,
}

impl Display for QueryField {
    /// Render one response field.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}={}", self.name, quote(&self.value))
    }
}

/// Render one query response as exact rows.
pub(super) fn response_rows(
    run: &QueryRun<'_>,
    call: &QueryCall,
    response: &QueryResponse,
) -> Result<ResponseRows, String> {
    let rows = match response {
        QueryResponse::Completion(response) => completion_rows(run, response)?,
        QueryResponse::CompletionDetails(response) => completion_details_rows(run, response)?,
        QueryResponse::Hover(response) => match &response.hover {
            Some(hover) => hover_rows(run, hover)?,
            None => none("hover"),
        },
        QueryResponse::SignatureHelp(response) => match &response.help {
            Some(help) => signature_rows(help)?,
            None => none("signature_help"),
        },
        QueryResponse::InlayHints(response) => inlay_rows(run, call, &response.hints)?,
        QueryResponse::CodeLenses(response) => {
            code_lens_rows(run, "code_lenses", &response.lenses)?
        }
        QueryResponse::FoldingRanges(response) => folding_rows(&response.ranges),
        QueryResponse::SemanticTokens(response) => {
            semantic_rows(run, "semantic_tokens", &response.tokens)?
        }
        QueryResponse::SemanticTokensRange(response) => {
            semantic_rows(run, "semantic_tokens_range", &response.tokens)?
        }
        QueryResponse::Outline(response) => outline_rows(run, &response.symbols)?,
        QueryResponse::SearchSymbols(response) => search_symbols_rows(run, &response.symbols)?,
        QueryResponse::Links(response) => link_rows(run, &response.links)?,
        QueryResponse::Highlight(response) => {
            let rows = response
                .highlights
                .iter()
                .map(|highlight| {
                    Ok(QueryRow::new("highlight.range")
                        .field("range", run.format_span(highlight.range)?)
                        .field("kind", enum_name(highlight.kind)))
                })
                .collect::<Result<Vec<_>, String>>()?;

            rows_or_none(rows, "highlight")
        }
        QueryResponse::SelectionRanges(response) => selection_rows(run, call, &response.ranges)?,
        QueryResponse::GotoDefinition(response) => {
            navigation_rows(run, "goto_definition", &response.targets)?
        }
        QueryResponse::GotoDeclaration(response) => {
            navigation_rows(run, "goto_declaration", &response.targets)?
        }
        QueryResponse::GotoTypeDefinition(response) => {
            navigation_rows(run, "goto_type_definition", &response.targets)?
        }
        QueryResponse::GotoImplementation(response) => {
            navigation_rows(run, "goto_implementation", &response.targets)?
        }
        QueryResponse::FindReferences(response) => {
            let rows = response
                .references
                .iter()
                .map(|reference| {
                    // require at least one exact identity for every occurrence
                    if reference.symbols.is_empty() {
                        return Err("query reference occurrence has no symbols".to_string());
                    }
                    let symbols = reference
                        .symbols
                        .iter()
                        .map(|symbol| {
                            run.format_symbol(*symbol, reference.target.module.profile_id)
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join(",");

                    let symbol_field = if reference.symbols.len() == 1 {
                        "symbol"
                    } else {
                        "symbols"
                    };

                    Ok(QueryRow::new("find_references.reference")
                        .with_target(run, &reference.target)?
                        .field(symbol_field, symbols))
                })
                .collect::<Result<Vec<_>, String>>()?;

            rows_or_none(rows, "find_references")
        }
        QueryResponse::CallItem(response) => match &response.item {
            Some(item) => call_item_rows(run, item)?,
            None => none("call_item"),
        },
        QueryResponse::IncomingCalls(response) => incoming_call_rows(run, &response.calls)?,
        QueryResponse::OutgoingCalls(response) => outgoing_call_rows(run, &response.calls)?,
        QueryResponse::TypeItem(response) => match &response.item {
            Some(item) => vec![type_item_row(run, "type_item.item", item)?],
            None => none("type_item"),
        },
        QueryResponse::Supertypes(response) => {
            type_item_rows(run, "supertypes", "supertypes.item", &response.items)?
        }
        QueryResponse::Subtypes(response) => {
            type_item_rows(run, "subtypes", "subtypes.item", &response.items)?
        }
        QueryResponse::Decorators(response) => decorator_rows(run, &response.decorators)?,
        QueryResponse::RenameTarget(response) => match &response.target {
            Some(rename_target) => {
                // require at least one exact identity for the rename occurrence
                if rename_target.symbols.is_empty() {
                    return Err("query rename target has no symbols".to_string());
                }
                let symbols = rename_target
                    .symbols
                    .iter()
                    .map(|symbol| {
                        run.format_symbol(*symbol, rename_target.target.module.profile_id)
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",");
                let symbol_field = if rename_target.symbols.len() == 1 {
                    "symbol"
                } else {
                    "symbols"
                };

                vec![
                    QueryRow::new("rename_target.target")
                        .field("placeholder", &rename_target.placeholder)
                        .with_target(run, &rename_target.target)?
                        .field(symbol_field, symbols),
                ]
            }
            None => none("rename_target"),
        },
        QueryResponse::CodeActions(response) => code_action_rows(run, &response.actions)?,
        QueryResponse::Rename(response) => edit_rows(run, "rename", response.edit.as_ref())?,
        QueryResponse::RenameFiles(response) => {
            edit_rows(run, "rename_files", response.edit.as_ref())?
        }
        QueryResponse::ExtractVariable(response) => {
            edit_rows(run, "extract_variable", response.edit.as_ref())?
        }
        QueryResponse::Inline(response) => edit_rows(run, "inline", response.edit.as_ref())?,
    };

    // require one explicit row even for an empty result
    if rows.is_empty() {
        return Err("query response produced no rows".to_string());
    }

    Ok(ResponseRows::new(rows))
}

/// Render completion rows and their additional edits.
fn completion_rows(
    run: &QueryRun<'_>,
    response: &CompletionResponse,
) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // record truncated result lists before their items
    if response.is_incomplete {
        rows.push(QueryRow::new("completion.list").field("incomplete", "true"));
    }

    // transcribe entries in response order
    for (entry_index, entry) in response.entries.iter().enumerate() {
        let matches = entry
            .match_positions
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",");
        rows.push(
            QueryRow::new("completion.item")
                .field("label", &entry.label)
                .field("kind", enum_name(entry.kind))
                .field("replace", run.format_span(entry.edit.span)?)
                .optional("suffix", entry.label_suffix.as_deref())
                .optional("description", entry.description.as_deref())
                .optional(
                    "insert",
                    (entry.edit.new_text != entry.label).then_some(entry.edit.new_text.as_str()),
                )
                .flag("snippet", entry.edit.is_snippet)
                .flag("preselect", entry.preselect)
                .flag("deprecated", entry.is_deprecated)
                .flag("auto_import", entry.is_auto_import)
                .optional("matches", (!matches.is_empty()).then_some(matches)),
        );

        // retain exact completion edits in compact list fixtures
        let additional_edits = match entry.details.as_ref() {
            Some(CompletionEntryDetails::Deferred(request)) => &request.additional_edits,
            Some(CompletionEntryDetails::Eager(response)) => &response.additional_edits,
            None => continue,
        };
        for patch in additional_edits {
            rows.push(patch_row(
                run,
                "completion.additional_edit",
                Some(("item", entry_index)),
                patch,
            )?);
        }
    }

    Ok(rows_or_none(rows, "completion"))
}

/// Render expanded completion fields and their additional edits.
fn completion_details_rows(
    run: &QueryRun<'_>,
    response: &CompletionDetailsResponse,
) -> Result<Vec<QueryRow>, String> {
    let mut rows = vec![
        QueryRow::new("completion_details.item")
            .optional("declaration", response.declaration.as_deref())
            .optional("documentation", response.documentation.as_deref()),
    ];

    // render every additional edit in source order
    for patch in &response.additional_edits {
        rows.push(patch_row(
            run,
            "completion_details.additional_edit",
            None,
            patch,
        )?);
    }

    Ok(rows)
}

/// Render every declaration in one hover result.
fn hover_rows(run: &QueryRun<'_>, hover: &Hover) -> Result<Vec<QueryRow>, String> {
    let mut rows =
        Vec::with_capacity(hover.items.len() + usize::from(hover.documentation.is_some()));

    // render documentation attached directly to the hovered source node
    if let Some(documentation) = &hover.documentation {
        rows.push(
            QueryRow::new("hover.documentation")
                .field("text", documentation)
                .field("range", run.format_span(hover.range)?),
        );
    }

    for (index, item) in hover.items.iter().enumerate() {
        rows.push(
            QueryRow::new("hover.item")
                .field("index", index.to_string())
                .field("declaration", &item.declaration)
                .optional("type", item.selected_type.as_deref())
                .optional("documentation", item.documentation.as_deref())
                .with_target(run, &item.target)?
                .field("range", run.format_span(hover.range)?),
        );
    }

    Ok(rows)
}

/// Render signature and parameter rows.
fn signature_rows(help: &SignatureHelp) -> Result<Vec<QueryRow>, String> {
    let active_signature = help.signatures.get(help.active_signature).ok_or_else(|| {
        format!(
            "query signature help selects signature {}, but returned {} signatures",
            help.active_signature,
            help.signatures.len()
        )
    })?;
    if let Some(active_parameter) = help.active_parameter
        && active_parameter >= active_signature.parameters.len()
    {
        return Err(format!(
            "query signature help selects parameter {}, but active signature has {} parameters",
            active_parameter,
            active_signature.parameters.len()
        ));
    }
    let mut rows = Vec::new();

    // transcribe signatures and parameters in response order
    for (signature_index, signature) in help.signatures.iter().enumerate() {
        rows.push(
            QueryRow::new("signature_help.signature")
                .field("index", signature_index.to_string())
                .field("label", &signature.label)
                .optional("documentation", signature.documentation.as_deref())
                .flag("active", signature_index == help.active_signature),
        );

        for (parameter_index, parameter) in signature.parameters.iter().enumerate() {
            rows.push(
                QueryRow::new("signature_help.parameter")
                    .field("signature", signature_index.to_string())
                    .field("index", parameter_index.to_string())
                    .field("label", &parameter.label)
                    .optional("documentation", parameter.documentation.as_deref())
                    .flag(
                        "active",
                        signature_index == help.active_signature
                            && Some(parameter_index) == help.active_parameter,
                    ),
            );
        }
    }

    Ok(rows)
}

/// Render inlay hint rows.
fn inlay_rows(
    run: &QueryRun<'_>,
    call: &QueryCall,
    hints: &[InlayHint],
) -> Result<Vec<QueryRow>, String> {
    let QueryCall::InlayHints { range, .. } = call else {
        return Err("inlay hint response has a mismatched query call".to_string());
    };
    let rows = hints
        .iter()
        .map(|hint| {
            Ok(QueryRow::new("inlay_hints.hint")
                .field("position", run.format_position(&range.file, hint.position)?)
                .field("label", &hint.label)
                .field("kind", enum_name(hint.kind))
                .flag("padding_left", hint.padding_left)
                .flag("padding_right", hint.padding_right))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, "inlay_hints"))
}

/// Render code lens rows.
fn code_lens_rows(
    run: &QueryRun<'_>,
    method: &str,
    lenses: &[CodeLens],
) -> Result<Vec<QueryRow>, String> {
    let rows = lenses
        .iter()
        .map(|lens| code_lens_row(run, method, lens))
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, method))
}

/// Render one code lens row.
fn code_lens_row(run: &QueryRun<'_>, method: &str, lens: &CodeLens) -> Result<QueryRow, String> {
    let row = QueryRow::new(format!("{method}.lens")).field("range", run.format_span(lens.range)?);
    let row = match &lens.action {
        CodeLensAction::References { count } => row
            .field("action", "references")
            .field("count", count.to_string()),
        CodeLensAction::Implementations { count } => row
            .field("action", "implementations")
            .field("count", count.to_string()),
    };

    Ok(row)
}

/// Render folding range rows.
fn folding_rows(ranges: &[FoldingRange]) -> Vec<QueryRow> {
    let rows = ranges
        .iter()
        .map(|range| {
            QueryRow::new("folding_ranges.range")
                .field("lines", format!("{}..{}", range.start_line, range.end_line))
                .optional(
                    "start",
                    range.start_character.map(|value| value.to_string()),
                )
                .optional("end", range.end_character.map(|value| value.to_string()))
                .optional("kind", range.kind.map(enum_name))
                .optional("collapsed", range.collapsed_text.as_deref())
        })
        .collect();

    rows_or_none(rows, "folding_ranges")
}

/// Render semantic token rows.
fn semantic_rows(
    run: &QueryRun<'_>,
    method: &str,
    tokens: &[SemanticToken],
) -> Result<Vec<QueryRow>, String> {
    let rows = tokens
        .iter()
        .map(|token| {
            let modifiers = semantic_modifiers(token.modifiers)?;

            Ok(QueryRow::new(format!("{method}.token"))
                .field("range", run.format_span(token.span)?)
                .field("type", enum_name(token.token_type))
                .optional(
                    "modifiers",
                    (!modifiers.is_empty()).then(|| modifiers.join(",")),
                ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, method))
}

/// Return semantic modifier names in protocol bit order.
fn semantic_modifiers(modifiers: SemanticTokenModifiers) -> Result<Vec<&'static str>, String> {
    let entries = [
        (SemanticTokenModifiers::DECLARATION, "declaration"),
        (SemanticTokenModifiers::READONLY, "readonly"),
        (SemanticTokenModifiers::STATIC, "static"),
        (SemanticTokenModifiers::DEPRECATED, "deprecated"),
        (SemanticTokenModifiers::ABSTRACT, "abstract"),
        (SemanticTokenModifiers::ASYNC, "async"),
        (SemanticTokenModifiers::MODIFICATION, "modification"),
        (SemanticTokenModifiers::DOCUMENTATION, "documentation"),
        (SemanticTokenModifiers::DEFAULT_LIBRARY, "default_library"),
    ];

    let known_bits = entries
        .iter()
        .fold(0, |bits, (modifier, _)| bits | modifier.bits());
    let unknown_bits = modifiers.bits() & !known_bits;
    if unknown_bits != 0 {
        return Err(format!(
            "query semantic token has unknown modifier bits {unknown_bits:#x}"
        ));
    }

    let names = entries
        .into_iter()
        .filter_map(|(flag, name)| modifiers.contains(flag).then_some(name))
        .collect();

    Ok(names)
}

/// Render hierarchical outline rows in preorder.
fn outline_rows(run: &QueryRun<'_>, symbols: &[OutlineSymbol]) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // preserve the hierarchy and sibling order
    for symbol in symbols {
        push_outline_rows(run, symbol, 0, &mut rows)?;
    }

    Ok(rows_or_none(rows, "outline"))
}

/// Append one outline subtree.
fn push_outline_rows(
    run: &QueryRun<'_>,
    symbol: &OutlineSymbol,
    depth: usize,
    rows: &mut Vec<QueryRow>,
) -> Result<(), String> {
    rows.push(
        QueryRow::new("outline.symbol")
            .field("depth", depth.to_string())
            .field("name", &symbol.name)
            .field("kind", enum_name(symbol.kind))
            .optional("detail", symbol.detail.as_deref())
            .field("range", run.format_span(symbol.range)?)
            .field("selection", run.format_span(symbol.selection_range)?),
    );

    // append children immediately after their parent
    for child in &symbol.children {
        push_outline_rows(run, child, depth + 1, rows)?;
    }

    Ok(())
}

/// Render workspace symbol rows.
fn search_symbols_rows(
    run: &QueryRun<'_>,
    symbols: &[SearchSymbol],
) -> Result<Vec<QueryRow>, String> {
    let rows = symbols
        .iter()
        .map(|symbol| {
            Ok(QueryRow::new("search_symbols.symbol")
                .field("name", &symbol.name)
                .field("kind", enum_name(symbol.kind))
                .optional("container", symbol.container.as_deref())
                .with_target(run, &symbol.target)?
                .field(
                    "symbol",
                    run.format_named_symbol(
                        symbol.symbol_id,
                        symbol.target.module.profile_id,
                        &symbol.name,
                    )?,
                ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, "search_symbols"))
}

/// Render document link rows.
fn link_rows(run: &QueryRun<'_>, links: &[Link]) -> Result<Vec<QueryRow>, String> {
    let rows = links
        .iter()
        .map(|link| {
            let row = QueryRow::new("links.link")
                .field("range", run.format_span(link.range)?)
                .field("path", run.format_file(link.target)?);

            Ok(row)
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, "links"))
}

/// Render every selection chain.
fn selection_rows(
    run: &QueryRun<'_>,
    call: &QueryCall,
    ranges: &[SelectionRange],
) -> Result<Vec<QueryRow>, String> {
    let QueryCall::SelectionRanges { positions } = call else {
        return Err("selection range response has a mismatched query call".to_string());
    };
    if ranges.len() != positions.len() {
        return Err(format!(
            "selection range query returned {} chains for {} positions",
            ranges.len(),
            positions.len()
        ));
    }
    let mut rows = Vec::new();

    // preserve requested-position and inner-to-outer chain order
    for (selection, range) in ranges.iter().enumerate() {
        let expected_path = &positions[selection].file;
        let mut depth = 0;
        let mut current = Some(range);
        while let Some(range) = current {
            let path = run.declared_path(range.range.file)?;
            if path != expected_path {
                return Err(format!(
                    "selection range chain {selection} names '{}', expected '{}'",
                    path.display(),
                    expected_path.display()
                ));
            }
            rows.push(
                QueryRow::new("selection_ranges.range")
                    .field("selection", selection.to_string())
                    .field("depth", depth.to_string())
                    .field("range", run.format_span(range.range)?),
            );
            current = range.parent.as_deref();
            depth += 1;
        }
    }

    Ok(rows_or_none(rows, "selection_ranges"))
}

/// Render ordered navigation targets.
fn navigation_rows(
    run: &QueryRun<'_>,
    method: &str,
    targets: &[NavigationTarget],
) -> Result<Vec<QueryRow>, String> {
    let rows = targets
        .iter()
        .map(|navigation| {
            run.require_module(navigation.origin.module, navigation.origin.span.file)?;
            let origin = run.format_span(navigation.origin.span)?;

            Ok(QueryRow::new(format!("{method}.target"))
                .field("origin", origin)
                .with_target(run, &navigation.target)?
                .field(
                    "symbol",
                    run.format_symbol(navigation.symbol_id, navigation.target.module.profile_id)?,
                ))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, method))
}

/// Render one standalone call item and its target.
fn call_item_rows(run: &QueryRun<'_>, item: &CallItem) -> Result<Vec<QueryRow>, String> {
    Ok(vec![call_item_row(run, "call_item.item", None, item)?])
}

/// Render incoming call rows and call-site ranges.
fn incoming_call_rows(run: &QueryRun<'_>, calls: &[IncomingCall]) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // preserve caller and call-site order
    for (call_index, call) in calls.iter().enumerate() {
        rows.push(call_item_row(
            run,
            "incoming_calls.call",
            Some(call_index),
            &call.from,
        )?);
        for range in &call.from_ranges {
            rows.push(
                QueryRow::new("incoming_calls.site")
                    .field("call", call_index.to_string())
                    .field("range", run.format_span(*range)?),
            );
        }
    }

    Ok(rows_or_none(rows, "incoming_calls"))
}

/// Render outgoing call rows and call-site ranges.
fn outgoing_call_rows(run: &QueryRun<'_>, calls: &[OutgoingCall]) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // preserve callee and call-site order
    for (call_index, call) in calls.iter().enumerate() {
        rows.push(call_item_row(
            run,
            "outgoing_calls.call",
            Some(call_index),
            &call.to,
        )?);
        for range in &call.from_ranges {
            rows.push(
                QueryRow::new("outgoing_calls.site")
                    .field("call", call_index.to_string())
                    .field("range", run.format_span(*range)?),
            );
        }
    }

    Ok(rows_or_none(rows, "outgoing_calls"))
}

/// Render one call hierarchy item.
fn call_item_row(
    run: &QueryRun<'_>,
    noun: &str,
    index: Option<usize>,
    item: &CallItem,
) -> Result<QueryRow, String> {
    Ok(QueryRow::new(noun)
        .optional("index", index.map(|index| index.to_string()))
        .field("name", &item.name)
        .field("kind", enum_name(item.kind))
        .optional("signature", item.signature.as_deref())
        .with_target(run, &item.target)?
        .field(
            "symbol",
            run.format_symbol(item.symbol_id, item.target.module.profile_id)?,
        ))
}

/// Render type hierarchy item rows.
fn type_item_rows(
    run: &QueryRun<'_>,
    method: &str,
    noun: &str,
    items: &[TypeItem],
) -> Result<Vec<QueryRow>, String> {
    let rows = items
        .iter()
        .map(|item| type_item_row(run, noun, item))
        .collect::<Result<Vec<_>, String>>()?;

    Ok(rows_or_none(rows, method))
}

/// Render one type hierarchy item.
fn type_item_row(run: &QueryRun<'_>, noun: &str, item: &TypeItem) -> Result<QueryRow, String> {
    Ok(QueryRow::new(noun)
        .field("name", &item.name)
        .field("kind", enum_name(item.kind))
        .optional("generics", item.generics.as_deref())
        .with_target(run, &item.target)?
        .field(
            "symbol",
            run.format_symbol(item.symbol_id, item.target.module.profile_id)?,
        ))
}

/// Render decorator rows.
fn decorator_rows(
    run: &QueryRun<'_>,
    decorators: &[DecoratorItem],
) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // preserve decorator applications and their paired owners
    for (index, decorator) in decorators.iter().enumerate() {
        let row = QueryRow::new("decorators.application")
            .field("index", index.to_string())
            .optional("name", decorator.name.as_deref());
        let row = match decorator.declaration {
            DecoratorTarget::LanguageItem { item, .. } => row
                .field("role", "language_item")
                .field("language_item", enum_name(item)),
            DecoratorTarget::Symbol { symbol } => row.field("role", "symbol").field(
                "symbol",
                run.format_symbol(symbol, decorator.application.module.profile_id)?,
            ),
        };
        rows.push(row.with_target(run, &decorator.application)?.field(
            "node",
            run.format_node(
                decorator.application_node.into_any(),
                decorator.application.module.profile_id,
            )?,
        ));

        rows.push(
            QueryRow::new("decorators.owner")
                .field("index", index.to_string())
                .with_target(run, &decorator.owner)?
                .field(
                    "node",
                    run.format_node(decorator.owner_node, decorator.owner.module.profile_id)?,
                ),
        );
    }

    Ok(rows_or_none(rows, "decorators"))
}

/// Render code action rows and exact patches.
fn code_action_rows(run: &QueryRun<'_>, actions: &[CodeAction]) -> Result<Vec<QueryRow>, String> {
    let mut rows = Vec::new();

    // preserve action, file, and patch order
    for (action_index, action) in actions.iter().enumerate() {
        rows.push(
            QueryRow::new("code_actions.action")
                .field("index", action_index.to_string())
                .field("title", &action.title)
                .field("kind", enum_name(action.kind))
                .optional("applicability", action.applicability.map(enum_name))
                .flag("preferred", action.is_preferred),
        );

        // retain every exact diagnostic addressed by this action
        for diagnostic in &action.diagnostics {
            let location = match diagnostic.primary.target {
                DiagnosticTarget::Span(span) => run.format_span(span)?,
                DiagnosticTarget::File(file) => run.format_file(file)?,
            };
            rows.push(
                QueryRow::new("code_actions.diagnostic")
                    .field("action", action_index.to_string())
                    .field("id", &diagnostic.id)
                    .field("location", location)
                    .optional("message", diagnostic.primary.message.as_deref()),
            );
        }
        rows.extend(patch_set_rows(
            run,
            "code_actions.patch",
            Some(("action", action_index)),
            &action.patches,
        )?);
    }

    Ok(rows_or_none(rows, "code_actions"))
}

/// Render every patch in one exact patch set.
fn patch_set_rows(
    run: &QueryRun<'_>,
    noun: &str,
    owner: Option<(&str, usize)>,
    patches: &PatchSet,
) -> Result<Vec<QueryRow>, String> {
    // reject present edits that do not change source
    if patches.is_empty() {
        return Err("query response contains an empty patch set".to_string());
    }
    let mut rows = Vec::new();
    let mut files = HashSet::new();

    // retain protocol file and patch order
    for file in &patches.files {
        if !files.insert(file.file) {
            return Err(format!(
                "query patch set names file {:?} more than once",
                file.file
            ));
        }
        if file.patches.is_empty() {
            return Err(format!(
                "query patch set contains an empty file patch for {:?}",
                file.file
            ));
        }

        for patch in &file.patches {
            if patch.span.file != file.file {
                return Err(format!(
                    "query patch span names {:?}, file patch names {:?}",
                    patch.span.file, file.file
                ));
            }
            rows.push(patch_row(run, noun, owner, patch)?);
        }
    }

    Ok(rows)
}

/// Render one optional edit as exact patches.
fn edit_rows(
    run: &QueryRun<'_>,
    method: &str,
    edit: Option<&PatchSet>,
) -> Result<Vec<QueryRow>, String> {
    let Some(edit) = edit else {
        return Ok(none(method));
    };
    let rows = patch_set_rows(run, &format!("{method}.patch"), None, edit)?;

    Ok(rows)
}

/// Render one exact source patch.
fn patch_row(
    run: &QueryRun<'_>,
    noun: &str,
    owner: Option<(&str, usize)>,
    patch: &Patch,
) -> Result<QueryRow, String> {
    let row = QueryRow::new(noun);
    let row = match owner {
        Some((name, index)) => row.field(name, index.to_string()),
        None => row,
    };

    Ok(row
        .field("range", run.format_span(patch.span)?)
        .field("text", &patch.new_text))
}

/// Return rows or one explicit empty response row.
fn rows_or_none(rows: Vec<QueryRow>, method: &str) -> Vec<QueryRow> {
    if rows.is_empty() { none(method) } else { rows }
}

/// Return one explicit empty response row.
fn none(method: &str) -> Vec<QueryRow> {
    vec![QueryRow::new(format!("{method}.none"))]
}

/// Return one lower snake case enum name.
fn enum_name(value: impl Debug) -> String {
    snake_case(&format!("{value:?}"))
}

/// Convert one upper camel case name to lower snake case.
fn snake_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len());

    // insert separators before upper-case words
    for (index, character) in value.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            result.push('_');
        }
        result.extend(character.to_lowercase());
    }

    result
}

/// Quote one row value only when its contents require quoting.
fn quote(value: &str) -> String {
    let is_plain = !value.is_empty()
        && !value.ends_with(':')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '_' | '.' | '/' | '#' | '@' | ':' | '-' | ',')
        });
    if is_plain {
        return value.to_string();
    }

    format!("{value:?}")
}
