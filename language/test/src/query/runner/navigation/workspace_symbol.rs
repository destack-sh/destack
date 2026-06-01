use destack_query::{SymbolKind, WorkspaceSymbol};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::snapshot::{compare_snapshot_lines, looks_like_snapshot};
use crate::query::runner::span::{file_for, format_span_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a workspace_symbols test.
///
/// Tests that workspace symbol search returns an exact symbol list.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    // resolve the expectation for this test
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
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

    let workspace = session.workspace_context();
    let symbols = workspace.workspace_symbols(query_str, 1000);
    let source_symbols = symbols.as_slice();

    let expected = exp.content.trim();

    // empty expectation is an error
    if expected.is_empty() {
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return CaseResult::Failed {
            message: format!(
                "workspace_symbols expectation is empty, query '{query_str}' returned: {names:?}"
            ),
        };
    }

    // treat <none> as an explicit empty result expectation
    if expected == "<none>" {
        if source_symbols.is_empty() {
            return CaseResult::Passed;
        }

        let actual_snapshot = format_workspace_symbol_snapshot(session, source_symbols).join("\n");
        return CaseResult::Failed {
            message: format!(
                "workspace_symbols for '{query_str}' expected none, got:\n{actual_snapshot}"
            ),
        };
    }

    // validate invariants before checking expectations
    if let Err(message) = validate_workspace_symbol_invariants(session, source_symbols) {
        return CaseResult::Failed { message };
    }

    // prefer protocol shaped snapshots when the expectation is structured
    if looks_like_snapshot(expected, &["(", "range=", "file="]) {
        return compare_snapshot_lines(
            &format!("workspace_symbols snapshot for '{query_str}'"),
            &format_workspace_symbol_snapshot(session, source_symbols),
            expected,
        );
    }

    // require an exact simple name list when no structured snapshot is provided
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let actual_names: Vec<&str> = source_symbols.iter().map(|s| s.name.as_str()).collect();

    if actual_names != expected_names {
        return CaseResult::Failed {
            message: format!(
                "workspace_symbols name list mismatch for '{query_str}'\n\nexpected: {expected_names:?}\nactual:   {actual_names:?}"
            ),
        };
    }

    CaseResult::Passed
}

/// Validate basic workspace symbol invariants.
fn validate_workspace_symbol_invariants(
    session: &QueryTestSession,
    symbols: &[WorkspaceSymbol],
) -> Result<(), String> {
    // collect invariant violations
    let mut errors = Vec::new();

    // ensure ranges are within known sources
    for symbol in symbols {
        // resolve the file for the symbol
        let Some(file) = file_for(session, symbol.target.span.file) else {
            errors.push(format!(
                "workspace symbol file {:?} not found",
                symbol.target.span.file
            ));
            continue;
        };
        let source_len = u32::try_from(file.source.len()).unwrap_or(u32::MAX);

        // validate the span order
        if symbol.target.span.start > symbol.target.span.end {
            errors.push(format!(
                "{}: symbol '{}' has invalid range {:?}",
                file.name, symbol.name, symbol.target.span
            ));
        }

        // validate span bounds
        if symbol.target.span.end > source_len {
            errors.push(format!(
                "{}: symbol '{}' range end {} exceeds source length {}",
                file.name, symbol.name, symbol.target.span.end, source_len
            ));
        }
    }

    // ensure there are no duplicate symbol locations
    for (index, left) in symbols.iter().enumerate() {
        for right in symbols.iter().skip(index + 1) {
            if left.name == right.name
                && left.kind == right.kind
                && left.target.span.file == right.target.span.file
                && left.target.span.start == right.target.span.start
                && left.target.span.end == right.target.span.end
            {
                errors.push(format!("duplicate workspace symbol {left:?}"));
            }
        }
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

/// Format workspace symbols into a protocol shaped snapshot.
fn format_workspace_symbol_snapshot(
    session: &QueryTestSession,
    symbols: &[WorkspaceSymbol],
) -> Vec<String> {
    // build snapshot lines for each symbol
    let mut lines = Vec::new();
    for symbol in symbols {
        lines.push(format_workspace_symbol_line(session, symbol));
    }
    // return the formatted lines
    lines
}

/// Format a single workspace symbol line.
fn format_workspace_symbol_line(session: &QueryTestSession, symbol: &WorkspaceSymbol) -> String {
    // resolve the symbol kind and file name for the snapshot line
    let kind = symbol_kind_name(symbol.kind);
    let file_name = file_for(session, symbol.target.span.file)
        .map(|file| file.name.as_str())
        .unwrap_or("<unknown>");

    // format the symbol range as line and column data
    let range = format_span(session, symbol.target.span);
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

/// Format a symbol kind as a lowercase name.
fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    // map symbol kinds to names
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
