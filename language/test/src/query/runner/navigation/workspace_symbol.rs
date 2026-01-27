use destack_source::Span;
use destack_workspace::query;
use destack_workspace::query::{SymbolKind, WorkspaceSymbol};

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{file_for, format_span_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a workspace_symbols test.
///
/// Tests that workspace symbol search returns expected symbols.
/// Format: `query workspace_symbols "query"` with content showing expected count or symbol names.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no workspace_symbols expectation".to_string(),
        };
    };

    // the query string is the target (or first arg)
    let query_str = if exp.args.is_empty() {
        &exp.target
    } else {
        &exp.args[0]
    };
    let query_str = query_str.trim_matches('"');

    let symbols = query::workspace_symbols(&session.session, query_str, 1000);
    let source_symbols = symbols.as_slice();

    let expected = exp.content.trim();

    // empty expectation is an error
    if expected.is_empty() {
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return TestResult::Failed {
            message: format!(
                "workspace_symbols expectation is empty, query '{query_str}' returned: {names:?}"
            ),
        };
    }

    // check if we should verify count
    if let Ok(expected_count) = expected.parse::<usize>() {
        if source_symbols.len() != expected_count {
            let names: Vec<&str> = source_symbols.iter().map(|s| s.name.as_str()).collect();
            return TestResult::Failed {
                message: format!(
                    "workspace_symbols for '{query_str}' returned {} symbols ({names:?}), expected {expected_count}",
                    source_symbols.len()
                ),
            };
        }
        return TestResult::Passed;
    }

    // validate invariants before checking expectations
    if let Err(message) = validate_workspace_symbol_invariants(session, query_str, source_symbols) {
        return TestResult::Failed { message };
    }

    // prefer protocol shaped snapshots when the expectation is structured
    if is_snapshot_expectation(expected) {
        let actual_snapshot = normalize_expected_snapshot(
            &format_workspace_symbol_snapshot(session, source_symbols).join("\n"),
        );
        let expected_snapshot = normalize_expected_snapshot(expected);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "workspace_symbols snapshot mismatch for '{query_str}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
                ),
            };
        }

        return TestResult::Passed;
    }

    // check expected symbol names
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let actual_names: Vec<&str> = source_symbols.iter().map(|s| s.name.as_str()).collect();

    for name in &expected_names {
        if !actual_names.contains(name) {
            return TestResult::Failed {
                message: format!(
                    "workspace_symbols for '{query_str}' missing '{name}', got: {actual_names:?}"
                ),
            };
        }
    }

    TestResult::Passed
}

/// Validate basic workspace symbol invariants.
fn validate_workspace_symbol_invariants(
    session: &QueryTestSession,
    query: &str,
    symbols: &[WorkspaceSymbol],
) -> Result<(), String> {
    let mut errors = Vec::new();
    let query_lower = query.to_lowercase();

    // ensure ranges are within known sources and names match the query
    for symbol in symbols {
        let Some(file) = file_for(session, symbol.file) else {
            errors.push(format!("workspace symbol file {:?} not found", symbol.file));
            continue;
        };
        let source_len = u32::try_from(file.source.len()).unwrap_or(u32::MAX);

        if symbol.range.start > symbol.range.end {
            errors.push(format!(
                "{}: symbol '{}' has invalid range {:?}",
                file.name, symbol.name, symbol.range
            ));
        }

        if symbol.range.end > source_len {
            errors.push(format!(
                "{}: symbol '{}' range end {} exceeds source length {}",
                file.name, symbol.name, symbol.range.end, source_len
            ));
        }

        let name_lower = symbol.name.to_lowercase();
        if !query_lower.is_empty() && !name_lower.contains(&query_lower) {
            errors.push(format!(
                "{}: symbol '{}' does not match query '{}'",
                file.name, symbol.name, query
            ));
        }
    }

    // ensure there are no duplicate symbol locations
    for (index, left) in symbols.iter().enumerate() {
        for right in symbols.iter().skip(index + 1) {
            if left.name == right.name
                && left.kind == right.kind
                && left.file == right.file
                && left.range.start == right.range.start
                && left.range.end == right.range.end
            {
                errors.push(format!("duplicate workspace symbol {left:?}"));
            }
        }
    }

    // ensure results are sorted by the expected ranking keys
    let mut previous_key: Option<(u8, String, u32, u32, u32)> = None;
    for symbol in symbols {
        let name_lower = symbol.name.to_lowercase();
        let score = match_score(&name_lower, &query_lower).unwrap_or(u8::MAX);
        let key = (
            score,
            name_lower,
            symbol.file.0,
            symbol.range.start,
            symbol.range.end,
        );
        if let Some(prev) = &previous_key
            && key < *prev
        {
            errors.push(format!(
                "workspace symbols are not sorted: {prev:?} before {key:?}"
            ));
        }
        previous_key = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "workspace_symbols invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains('(') || line.contains("range=") || line.contains("file="))
}

/// Format workspace symbols into a protocol shaped snapshot.
fn format_workspace_symbol_snapshot(
    session: &QueryTestSession,
    symbols: &[WorkspaceSymbol],
) -> Vec<String> {
    let mut lines = Vec::new();
    for symbol in symbols {
        lines.push(format_workspace_symbol_line(session, symbol));
    }
    lines
}

/// Format a single workspace symbol line.
fn format_workspace_symbol_line(session: &QueryTestSession, symbol: &WorkspaceSymbol) -> String {
    // resolve the symbol kind and file name for the snapshot line
    let kind = symbol_kind_name(symbol.kind);
    let file_name = file_for(session, symbol.file)
        .map(|file| file.name.as_str())
        .unwrap_or("<unknown>");

    // format the symbol range as line and column data
    let range = format_span(session, symbol.range);
    if let Some(container) = &symbol.container {
        return format!(
            "{}({kind}) file={file_name} range={range} container={container}",
            symbol.name
        );
    }
    format!("{}({kind}) file={file_name} range={range}", symbol.name)
}

/// Format a span as a 1 based line and column range.
fn format_span(session: &QueryTestSession, span: Span) -> String {
    // resolve the source text for this span's file
    let source = source_for_file(session, span.file);

    // format the span using the shared helper
    format_span_line_col(source, span)
}

/// Score how well a symbol name matches the query.
fn match_score(name_lower: &str, query_lower: &str) -> Option<u8> {
    if query_lower.is_empty() {
        return Some(2);
    }

    if name_lower == query_lower {
        return Some(0);
    }

    if name_lower.starts_with(query_lower) {
        return Some(1);
    }

    if name_lower.contains(query_lower) {
        return Some(2);
    }

    None
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
