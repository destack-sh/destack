use destack_query as query;
use destack_query::{Completion, CompletionTrigger};
use destack_source::Span;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{ExpectedCompletion, QueryExpectation, QueryTestSession};

/// Run a completion test.
///
/// Supports both markdown format (query block) and inline format (@completion).
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // if we have a markdown expectation, use it
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: use inline @completion expectations
    let expectations = &session.markers.expectations.completions;

    if expectations.is_empty() {
        return TestResult::Skipped {
            reason: "no completion expectations defined".to_string(),
        };
    }

    for (cursor_idx, expected) in expectations {
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found in source"),
            };
        };

        let completions = query::completions(
            &session.session,
            session.file_id,
            cursor.offset,
            CompletionTrigger::Invoked,
        );

        // validate completion invariants before comparisons
        if let Err(message) = validate_completion_invariants(session, &completions) {
            return TestResult::Failed { message };
        }

        // check expected completions are present
        for exp in expected {
            let found = completions
                .iter()
                .any(|item| item.label == exp.label && kind_to_str(&item.kind) == exp.kind);

            if !found {
                let actual: Vec<_> = completions
                    .iter()
                    .map(|i| format!("{}({})", i.label, kind_to_str(&i.kind)))
                    .collect();
                return TestResult::Failed {
                    message: format!(
                        "completion '{}({})' not found at ${cursor_idx}\nactual: {actual:?}",
                        exp.label, exp.kind
                    ),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation: `query completion $0` with content listing expected items.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve the target position, allowing file scoped targets
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // normalize the expectation content once
    let content = exp.content.trim();

    // empty expectations are not allowed
    if content.is_empty() {
        return TestResult::Failed {
            message: format!("completion expectation is empty at '{}'", exp.target),
        };
    }

    let completions = query::completions(
        &session.session,
        file_id,
        offset,
        CompletionTrigger::Invoked,
    );

    // validate completion invariants before comparisons
    if let Err(message) = validate_completion_invariants(session, &completions) {
        return TestResult::Failed { message };
    }

    // support snapshot expectations for stricter ordering assertions
    if is_snapshot_expectation(content) {
        // parse an optional top directive for prefix snapshot checks
        let (top_limit, snapshot_body) = parse_snapshot_top_directive(content);

        // format the actual completions into snapshot lines
        let mut actual_lines = format_completion_snapshot_lines(session, &completions);
        if let Some(limit) = top_limit {
            actual_lines.truncate(limit);
        }
        let actual_snapshot = normalize_expected_snapshot(&actual_lines.join("\n"));
        let expected_snapshot = normalize_expected_snapshot(&snapshot_body);

        if actual_snapshot == expected_snapshot {
            return TestResult::Passed;
        }

        return TestResult::Failed {
            message: format!(
                "completion snapshot mismatch at '{}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}",
                exp.target
            ),
        };
    }

    // parse expected completions from content
    let ParsedExpectations {
        expected,
        excluded,
        ordered,
    } = parse_completion_content(content);

    // skip only when no explicit expectations are present
    if expected.is_empty() && excluded.is_empty() {
        return TestResult::Skipped {
            reason: "no expected completions in query block".to_string(),
        };
    }

    // check expected completions are present
    for exp_item in &expected {
        let found = completions
            .iter()
            .any(|item| item.label == exp_item.label && kind_to_str(&item.kind) == exp_item.kind);

        if !found {
            let actual: Vec<_> = completions
                .iter()
                .map(|i| format!("{}: {}", i.label, kind_to_str(&i.kind)))
                .collect();
            return TestResult::Failed {
                message: format!(
                    "completion '{}: {}' not found\nactual: {:?}",
                    exp_item.label, exp_item.kind, actual
                ),
            };
        }
    }

    // check excluded completions are NOT present
    for exc_item in &excluded {
        let found = completions
            .iter()
            .any(|item| item.label == exc_item.label && kind_to_str(&item.kind) == exc_item.kind);

        if found {
            return TestResult::Failed {
                message: format!(
                    "completion '{}: {}' should NOT be present but was found",
                    exc_item.label, exc_item.kind
                ),
            };
        }
    }

    // check ordered completions appear in order
    if ordered.len() > 1 {
        let mut last_index = None;

        for item in &ordered {
            let Some(index) = completions.iter().position(|candidate| {
                candidate.label == item.label && kind_to_str(&candidate.kind) == item.kind
            }) else {
                return TestResult::Failed {
                    message: format!(
                        "ordered completion '{}: {}' not found",
                        item.label, item.kind
                    ),
                };
            };

            // fail when ordering is violated
            if let Some(previous) = last_index
                && index <= previous
            {
                return TestResult::Failed {
                    message: format!(
                        "completion order violated for '{}: {}'",
                        item.label, item.kind
                    ),
                };
            }

            last_index = Some(index);
        }
    }

    TestResult::Passed
}

/// Parsed completion expectations.
struct ParsedExpectations {
    /// Completions that should be present.
    expected: Vec<ExpectedCompletion>,
    /// Completions that should NOT be present.
    excluded: Vec<ExpectedCompletion>,
    /// Completions that should appear in order.
    ordered: Vec<ExpectedCompletion>,
}

/// Parse markdown list of completions: "- x: field\n- y: field".
/// Also supports exclusions with "! x: field" prefix.
fn parse_completion_content(content: &str) -> ParsedExpectations {
    let mut expected = Vec::new();
    let mut excluded = Vec::new();
    let mut ordered = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(order_line) = line.strip_prefix("order:") {
            for entry in order_line.split(',') {
                let entry = entry.trim();
                if entry.is_empty() {
                    continue;
                }

                if let Some(completion) = parse_completion_entry(entry) {
                    ordered.push(completion);
                }
            }
            continue;
        }

        // exclusion: "! x: field" or "- ! x: field"
        let is_exclusion = line.starts_with("! ") || line.starts_with("- ! ");
        let line = line
            .strip_prefix("- ! ")
            .or_else(|| line.strip_prefix("! "))
            .or_else(|| line.strip_prefix("- "));

        let Some(line) = line else { continue };
        let Some(completion) = parse_completion_entry(line) else {
            continue;
        };

        if is_exclusion {
            excluded.push(completion);
        } else {
            expected.push(completion);
        }
    }

    ParsedExpectations {
        expected,
        excluded,
        ordered,
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with('[') || line.contains("label=") || line.contains("sort="))
}

/// Parse an optional top directive for snapshot checks.
fn parse_snapshot_top_directive(content: &str) -> (Option<usize>, String) {
    let mut top_limit = None;
    let mut lines = Vec::new();

    // parse the first non empty line as a potential top directive
    let mut directive_consumed = false;
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !directive_consumed {
            let rest = trimmed.strip_prefix("top:");
            if let Some(rest) = rest {
                top_limit = rest.trim().parse::<usize>().ok();
                directive_consumed = true;
                continue;
            }
        }

        directive_consumed = true;
        lines.push(trimmed.to_string());
    }

    (top_limit, lines.join("\n"))
}

/// Validate completion invariants like label and edit bounds.
fn validate_completion_invariants(
    session: &QueryTestSession,
    completions: &[Completion],
) -> Result<(), String> {
    let mut errors = Vec::new();

    for completion in completions {
        // require non empty labels
        if completion.label.trim().is_empty() {
            errors.push("completion has empty label".to_string());
        }

        // require match positions to be sorted and in bounds
        if let Err(message) =
            validate_match_positions(&completion.label, &completion.match_positions)
        {
            errors.push(message);
        }

        // require additional edits to be in bounds for their files
        for edit in &completion.additional_text_edits {
            let source = source_for_file(session, edit.span.file);
            let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);
            validate_span_bounds(
                "completion additional_text_edit",
                edit.span,
                source_len,
                &mut errors,
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "completion invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate completion match positions.
fn validate_match_positions(label: &str, positions: &[usize]) -> Result<(), String> {
    let mut previous = None;
    for &position in positions {
        // reject match positions that exceed the label length
        if position >= label.len() {
            return Err(format!(
                "completion match position {position} out of bounds for '{label}'",
            ));
        }

        // reject match positions that move backwards
        if let Some(prev) = previous
            && position < prev
        {
            return Err(format!(
                "completion match positions are not sorted for '{label}'",
            ));
        }

        previous = Some(position);
    }

    Ok(())
}

/// Validate that a span is within the source bounds.
fn validate_span_bounds(label: &str, span: Span, source_len: u32, errors: &mut Vec<String>) {
    // ensure the span start does not exceed the end
    if span.start > span.end {
        errors.push(format!(
            "{label}: span start {} is after end {}",
            span.start, span.end,
        ));
    }

    // ensure the span end stays within the source bounds
    if span.end > source_len {
        errors.push(format!(
            "{label}: span end {} exceeds source length {}",
            span.end, source_len,
        ));
    }
}

/// Format completions into protocol shaped snapshot lines.
fn format_completion_snapshot_lines(
    session: &QueryTestSession,
    completions: &[Completion],
) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, completion) in completions.iter().enumerate() {
        lines.push(format_completion_line(session, index, completion));
    }

    lines
}

/// Format a single completion snapshot line.
fn format_completion_line(
    session: &QueryTestSession,
    index: usize,
    completion: &Completion,
) -> String {
    let kind = kind_to_str(&completion.kind);
    let sort_text = completion
        .sort_text
        .as_deref()
        .map(flatten_text)
        .unwrap_or_else(|| "<none>".to_string());
    let detail = completion
        .detail
        .as_deref()
        .map(flatten_text)
        .unwrap_or_else(|| "<none>".to_string());

    let edits = format_additional_edits(session, completion);

    format!(
        "[{index}] label={} kind={kind} sort={} sort_text={sort_text} detail={detail} edits={edits}",
        completion.label, completion.sort_order,
    )
}

/// Format additional edits for snapshot output.
fn format_additional_edits(session: &QueryTestSession, completion: &Completion) -> String {
    if completion.additional_text_edits.is_empty() {
        return "<none>".to_string();
    }

    let mut edits = Vec::new();
    for edit in &completion.additional_text_edits {
        let range = format_span_for_session(session, edit.span);
        let new_text = flatten_text(&edit.new_text).replace('"', "\\\"");
        edits.push(format!("{range}=>\"{new_text}\""));
    }

    edits.join("|")
}

/// Flatten multi-line text into a single line for snapshots.
fn flatten_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_completion_entry(line: &str) -> Option<ExpectedCompletion> {
    let (label, kind) = line.split_once(':')?;
    Some(ExpectedCompletion {
        label: label.trim().to_string(),
        kind: kind.trim().to_string(),
        detail: None,
    })
}

fn kind_to_str(kind: &query::CompletionKind) -> &'static str {
    match kind {
        query::CompletionKind::Text => "text",
        query::CompletionKind::Method => "method",
        query::CompletionKind::Function => "function",
        query::CompletionKind::Constructor => "constructor",
        query::CompletionKind::Field => "field",
        query::CompletionKind::Variable => "variable",
        query::CompletionKind::Class => "class",
        query::CompletionKind::Interface => "interface",
        query::CompletionKind::Module => "module",
        query::CompletionKind::Property => "property",
        query::CompletionKind::Unit => "unit",
        query::CompletionKind::Value => "value",
        query::CompletionKind::Enum => "enum",
        query::CompletionKind::Keyword => "keyword",
        query::CompletionKind::Snippet => "snippet",
        query::CompletionKind::Color => "color",
        query::CompletionKind::File => "file",
        query::CompletionKind::Reference => "reference",
        query::CompletionKind::Folder => "folder",
        query::CompletionKind::EnumMember => "enum_member",
        query::CompletionKind::Constant => "constant",
        query::CompletionKind::Struct => "struct",
        query::CompletionKind::Event => "event",
        query::CompletionKind::Operator => "operator",
        query::CompletionKind::TypeParameter => "type_parameter",
    }
}
