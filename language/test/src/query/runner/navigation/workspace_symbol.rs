use destack_query as query;
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

    let symbols = query::workspace_symbols(&session.session, query_str, 1000);
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
    if let Err(message) = validate_workspace_symbol_invariants(session, query_str, source_symbols) {
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
    query: &str,
    symbols: &[WorkspaceSymbol],
) -> Result<(), String> {
    // collect invariant violations
    let mut errors = Vec::new();
    let query = query.trim();

    // ensure ranges are within known sources and names match the query
    for symbol in symbols {
        // resolve the file for the symbol
        let Some(file) = file_for(session, symbol.file) else {
            errors.push(format!("workspace symbol file {:?} not found", symbol.file));
            continue;
        };
        let source_len = u32::try_from(file.source.len()).unwrap_or(u32::MAX);

        // validate the span order
        if symbol.range.start > symbol.range.end {
            errors.push(format!(
                "{}: symbol '{}' has invalid range {:?}",
                file.name, symbol.name, symbol.range
            ));
        }

        // validate span bounds
        if symbol.range.end > source_len {
            errors.push(format!(
                "{}: symbol '{}' range end {} exceeds source length {}",
                file.name, symbol.name, symbol.range.end, source_len
            ));
        }

        // ensure the symbol matches the query filter
        if score_symbol_match(&symbol.name, query).is_none() {
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
    let mut previous_key: Option<(std::cmp::Reverse<u32>, usize, String, u32, u32, u32)> = None;
    for symbol in symbols {
        // compute the score and name for sorting
        let name_lower = symbol.name.to_lowercase();
        let Some(score) = score_symbol_match(&symbol.name, query) else {
            errors.push(format!(
                "workspace symbol '{}' does not match query '{}' during ordering validation",
                symbol.name, query
            ));
            continue;
        };
        let key = (
            std::cmp::Reverse(score),
            symbol.name.len(),
            name_lower,
            symbol.file.0,
            symbol.range.start,
            symbol.range.end,
        );
        // record ordering violations
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
fn score_symbol_match(name: &str, query: &str) -> Option<u32> {
    // defer to shared scoring logic
    score_completion(name, query)
}

/// Score a candidate against a typed prefix.
fn score_completion(candidate: &str, prefix: &str) -> Option<u32> {
    // no filter: base score adjusted by length
    if prefix.is_empty() {
        return Some(score_no_filter(candidate));
    }

    // exact prefix match
    if candidate.starts_with(prefix) {
        return Some(score_exact_prefix(candidate));
    }

    // case insensitive prefix match
    if candidate.to_lowercase().starts_with(&prefix.to_lowercase()) {
        return Some(score_case_insensitive_prefix(candidate));
    }

    // word boundary match
    if word_boundary_match(candidate, prefix) {
        return Some(score_word_boundary(candidate));
    }

    // fuzzy subsequence match
    if fuzzy_subsequence_match(candidate, prefix) {
        return Some(score_fuzzy_subsequence(candidate, prefix));
    }

    None
}

/// Compute the score for an exact prefix match.
fn score_exact_prefix(candidate: &str) -> u32 {
    // return the base score for exact matches
    2000u32.saturating_sub(candidate.len() as u32)
}

/// Compute the score for a case insensitive prefix match.
fn score_case_insensitive_prefix(candidate: &str) -> u32 {
    // return the base score for case insensitive prefixes
    1800u32.saturating_sub(candidate.len() as u32)
}

/// Compute the score for a word boundary match.
fn score_word_boundary(candidate: &str) -> u32 {
    // return the base score for boundary matches
    1600u32.saturating_sub(candidate.len() as u32)
}

/// Compute the score for a fuzzy subsequence match.
fn score_fuzzy_subsequence(candidate: &str, prefix: &str) -> u32 {
    // compute the spread and apply penalties
    let spread = fuzzy_match_spread(candidate, prefix).unwrap_or(u32::MAX);
    1400u32
        .saturating_sub(spread)
        .saturating_sub(candidate.len() as u32)
}

/// Compute the score when no filter is applied.
fn score_no_filter(candidate: &str) -> u32 {
    // return the base score for unfiltered queries
    1000u32.saturating_sub(candidate.len() as u32)
}

/// Check if prefix matches word boundaries in candidate.
fn word_boundary_match(candidate: &str, prefix: &str) -> bool {
    // collect word boundary positions
    let boundaries: Vec<usize> = candidate
        .char_indices()
        .filter(|(i, c)| {
            *i == 0
                || c.is_uppercase()
                || (*i > 0 && candidate.as_bytes().get(i - 1) == Some(&b'_'))
        })
        .map(|(i, _)| i)
        .collect();

    // match prefix chars against boundary positions
    let prefix_chars: Vec<char> = prefix.chars().collect();
    let mut prefix_idx = 0;

    for &boundary in &boundaries {
        if prefix_idx >= prefix_chars.len() {
            break;
        }

        let Some(candidate_char) = candidate.chars().nth(boundary) else {
            continue;
        };

        if candidate_char.to_lowercase().next() == prefix_chars[prefix_idx].to_lowercase().next() {
            prefix_idx += 1;
        }
    }

    prefix_idx == prefix_chars.len()
}

/// Check if all prefix characters appear in candidate in order.
fn fuzzy_subsequence_match(candidate: &str, prefix: &str) -> bool {
    // return whether a fuzzy match exists
    fuzzy_match_spread(candidate, prefix).is_some()
}

/// Compute the spread between the first and last fuzzy match.
fn fuzzy_match_spread(candidate: &str, prefix: &str) -> Option<u32> {
    // normalize inputs for matching
    let candidate_lower = candidate.to_lowercase();
    let prefix_lower = prefix.to_lowercase();

    let mut candidate_iter = candidate_lower.char_indices();
    let mut first_pos: Option<usize> = None;
    let mut last_pos: usize = 0;

    for prefix_char in prefix_lower.chars() {
        // find next matching character
        let found = candidate_iter.find(|(_, c)| *c == prefix_char);

        match found {
            Some((pos, _)) => {
                if first_pos.is_none() {
                    first_pos = Some(pos);
                }
                last_pos = pos;
            }
            None => return None,
        }
    }

    let first = first_pos.unwrap_or(0);

    // return the spread between first and last match
    Some((last_pos - first) as u32)
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
