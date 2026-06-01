use destack_query::{DocumentSymbol, SymbolKind};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{compute_line_starts, offset_to_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_symbols test.
///
/// Tests that document symbols returns the expected symbols for a file.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no document_symbols expectation".to_string(),
        };
    };

    let expected = exp.content.trim();

    // empty expectation is an error: must specify expected symbols
    if expected.is_empty() {
        let ctx = session.primary_module_context();
        let symbols = ctx.document_symbols();
        let actual_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return CaseResult::Failed {
            message: format!(
                "document_symbols expectation is empty, but query returned: {actual_names:?}"
            ),
        };
    }

    // run the document symbols query once
    let ctx = session.primary_module_context();
    let symbols = ctx.document_symbols();

    // allow explicit empty snapshots
    if expected == "<none>" {
        return if symbols.is_empty() {
            CaseResult::Passed
        } else {
            let actual_snapshot = format_symbols_hierarchical(&symbols, 0).join("\n");
            CaseResult::Failed {
                message: format!("document_symbols expected no symbols, got:\n{actual_snapshot}"),
            }
        };
    }

    // read the source for span validation and formatting
    let source = source_for_file(session, session.file_id);

    // prefer protocol shaped snapshots when the expectation is structured
    if is_protocol_symbols_expectation(expected) {
        return run_protocol_symbols_expectation(&symbols, source, expected);
    }

    // compare the full hierarchical shape for legacy expectations too
    let actual_lines = format_symbols_hierarchical(&symbols, 0);
    let actual_snapshot = actual_lines.join("\n");
    let expected_snapshot = expected
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if actual_snapshot != expected_snapshot {
        return CaseResult::Failed {
            message: format!(
                "document_symbols mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
            ),
        };
    }

    CaseResult::Passed
}

/// Format symbols hierarchically with indentation.
fn format_symbols_hierarchical(symbols: &[DocumentSymbol], indent: usize) -> Vec<String> {
    let mut result = Vec::new();

    // walk the symbol tree depth first and accumulate indentation
    for symbol in symbols {
        result.push(format!("{}{}", " ".repeat(indent), symbol.name));
        result.extend(format_symbols_hierarchical(&symbol.children, indent + 2));
    }

    result
}

/// Run a protocol shaped document symbol expectation.
fn run_protocol_symbols_expectation(
    symbols: &[DocumentSymbol],
    source: &str,
    expected: &str,
) -> CaseResult {
    // validate protocol invariants before snapshot comparison
    if let Err(message) = validate_symbol_invariants(symbols, source) {
        return CaseResult::Failed { message };
    }

    // format the actual tree into a deterministic snapshot
    let actual_lines = format_symbols_snapshot(symbols, source, 0);
    let actual_snapshot = normalize_expected_snapshot(&actual_lines.join("\n"));

    // normalize the expected snapshot into the same newline separated form
    let expected_snapshot = normalize_expected_snapshot(expected);

    // require an exact snapshot match in protocol mode
    if actual_snapshot != expected_snapshot {
        return CaseResult::Failed {
            message: format!(
                "document_symbols snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
            ),
        };
    }

    CaseResult::Passed
}

/// Validate basic document symbol invariants.
fn validate_symbol_invariants(symbols: &[DocumentSymbol], source: &str) -> Result<(), String> {
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let mut errors = Vec::new();

    // validate each symbol against the source bounds
    for symbol in symbols {
        validate_symbol(symbol, None, source_len, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "document_symbols invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate a single symbol and its children.
fn validate_symbol(
    symbol: &DocumentSymbol,
    parent: Option<&DocumentSymbol>,
    source_len: u32,
    errors: &mut Vec<String>,
) {
    // validate the symbol's own ranges
    validate_span_bounds(&symbol.name, "range", symbol.range, source_len, errors);
    validate_span_bounds(
        &symbol.name,
        "selection",
        symbol.selection_range,
        source_len,
        errors,
    );
    validate_selection_within_range(symbol, errors);

    // validate parent child containment
    if let Some(parent_symbol) = parent {
        validate_child_within_parent(parent_symbol, symbol, errors);
    }

    // validate children recursively against this symbol
    for child in &symbol.children {
        validate_symbol(child, Some(symbol), source_len, errors);
    }
}

/// Validate that a span is within the source bounds.
fn validate_span_bounds(
    name: &str,
    label: &str,
    span: Span,
    source_len: u32,
    errors: &mut Vec<String>,
) {
    // ensure the span start does not exceed the end
    if span.start > span.end {
        errors.push(format!(
            "{name}: {label} start {} is after end {}",
            span.start, span.end
        ));
    }

    // ensure the span end stays within the source bounds
    if span.end > source_len {
        errors.push(format!(
            "{name}: {label} end {} exceeds source length {}",
            span.end, source_len
        ));
    }
}

/// Validate that selection_range is contained within range.
fn validate_selection_within_range(symbol: &DocumentSymbol, errors: &mut Vec<String>) {
    // ensure the selection range stays within the full symbol range
    if symbol.selection_range.start < symbol.range.start
        || symbol.selection_range.end > symbol.range.end
    {
        errors.push(format!(
            "{}: selection {:?} is outside range {:?}",
            symbol.name, symbol.selection_range, symbol.range
        ));
    }
}

/// Validate that a child range is contained within the parent range.
fn validate_child_within_parent(
    parent: &DocumentSymbol,
    child: &DocumentSymbol,
    errors: &mut Vec<String>,
) {
    // ensure the child range stays within the parent range
    if child.range.start < parent.range.start || child.range.end > parent.range.end {
        errors.push(format!(
            "{}: child '{}' range {:?} is outside parent range {:?}",
            parent.name, child.name, child.range, parent.range
        ));
    }
}

/// Decide whether a document symbol expectation is protocol shaped.
fn is_protocol_symbols_expectation(expected: &str) -> bool {
    // detect protocol snapshots by kind or range markers
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains('(') || line.contains("range=") || line.contains("selection="))
}

/// Format a document symbol tree into a protocol shaped snapshot.
fn format_symbols_snapshot(symbols: &[DocumentSymbol], source: &str, indent: usize) -> Vec<String> {
    // compute line starts once for consistent span formatting
    let line_starts = compute_line_starts(source);

    // allocate the snapshot lines
    let mut lines = Vec::new();

    // render each symbol with its range and selection range
    for symbol in symbols {
        let indent_prefix = " ".repeat(indent);
        let kind = symbol_kind_name(symbol.kind);
        let range = format_span(&line_starts, symbol.range);
        let selection = format_span(&line_starts, symbol.selection_range);
        lines.push(format!(
            "{indent_prefix}{}({kind}) range={range} selection={selection}",
            symbol.name
        ));
        lines.extend(format_symbols_snapshot(
            &symbol.children,
            source,
            indent + 2,
        ));
    }
    lines
}

/// Format a span as a 1 based line and column range.
fn format_span(line_starts: &[u32], span: Span) -> String {
    let start = offset_to_line_col(line_starts, span.start);
    let end = offset_to_line_col(line_starts, span.end);
    format!("{}:{}-{}:{}", start.0, start.1, end.0, end.1)
}

/// Format a symbol kind as a lowercase name.
fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::File => "file",
        SymbolKind::Module => "module",
        SymbolKind::Namespace => "namespace",
        SymbolKind::Package => "package",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Property => "property",
        SymbolKind::Field => "field",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Enum => "enum",
        SymbolKind::Interface => "interface",
        SymbolKind::Function => "function",
        SymbolKind::Variable => "variable",
        SymbolKind::Constant => "constant",
        SymbolKind::String => "string",
        SymbolKind::Number => "number",
        SymbolKind::Boolean => "boolean",
        SymbolKind::Array => "array",
        SymbolKind::Object => "object",
        SymbolKind::Key => "key",
        SymbolKind::Null => "null",
        SymbolKind::EnumMember => "enum_member",
        SymbolKind::Struct => "struct",
        SymbolKind::Event => "event",
        SymbolKind::Operator => "operator",
        SymbolKind::TypeParameter => "type_parameter",
    }
}
