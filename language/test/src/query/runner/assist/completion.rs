use destack_query as query;
use destack_query::{Completion, CompletionTrigger};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{
    compare_snapshot, looks_like_snapshot, parse_snapshot_top_directive,
};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// One exact expected completion item.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedCompletion {
    /// The completion label.
    label: String,
    /// The completion kind name.
    kind: String,
    /// The optional detail text.
    detail: Option<String>,
}

/// Run a completion test.
///
/// Supports both markdown format (query block) and inline format (@completion).
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no completion expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation: `query completion $0` with content listing expected items.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the target position, allowing file scoped targets
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // normalize the expectation content once
    let content = exp.content.trim();

    // empty expectations are not allowed
    if content.is_empty() {
        return CaseResult::Failed {
            message: format!("completion expectation is empty at '{}'", exp.target),
        };
    }

    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let completions = query::completions(&ctx, &workspace, offset, CompletionTrigger::Invoked);

    // validate completion invariants before comparisons
    if let Err(message) = validate_completion_invariants(session, &completions) {
        return CaseResult::Failed { message };
    }

    // support snapshot expectations for stricter ordering assertions
    if looks_like_snapshot(content, &["[", "label=", "sort="]) {
        // parse an optional top directive for prefix snapshot checks
        let (top_limit, snapshot_body) = parse_snapshot_top_directive(content);

        // format the actual completions into snapshot lines
        let mut actual_lines = format_completion_snapshot_lines(session, &completions);
        if let Some(limit) = top_limit {
            actual_lines.truncate(limit);
        }
        return compare_snapshot(
            &format!("completion snapshot at '{}'", exp.target),
            &actual_lines.join("\n"),
            &snapshot_body,
        );
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        return if completions.is_empty() {
            CaseResult::Passed
        } else {
            let actual = completions
                .iter()
                .map(ExpectedCompletion::from)
                .map(|completion| format_expected_completion(&completion))
                .collect::<Vec<_>>();
            CaseResult::Failed {
                message: format!(
                    "completion at '{}' expected none\n\nactual: {actual:?}",
                    exp.target
                ),
            }
        };
    }

    // parse the exact expected completion list
    let expected = match parse_completion_entries(content) {
        Ok(expected) => expected,
        Err(message) => return CaseResult::Failed { message },
    };

    // reject empty non snapshot expectations
    if expected.is_empty() {
        return CaseResult::Failed {
            message: format!("completion expectation is empty at '{}'", exp.target),
        };
    }

    // compare exact completion membership
    let actual = completions
        .iter()
        .map(ExpectedCompletion::from)
        .collect::<Vec<_>>();

    if actual != expected {
        let expected = expected
            .iter()
            .map(format_expected_completion)
            .collect::<Vec<_>>();
        let actual = actual
            .iter()
            .map(format_expected_completion)
            .collect::<Vec<_>>();
        return CaseResult::Failed {
            message: format!(
                "completion set mismatch at '{}'\n\nexpected: {expected:?}\nactual:   {actual:?}",
                exp.target
            ),
        };
    }

    CaseResult::Passed
}

/// Format one completion for failure messages.
fn format_expected_completion(completion: &ExpectedCompletion) -> String {
    format!("{}: {}", completion.label, completion.kind)
}

impl From<&Completion> for ExpectedCompletion {
    fn from(completion: &Completion) -> Self {
        Self {
            label: completion.label.clone(),
            kind: kind_to_str(&completion.kind).to_string(),
            detail: None,
        }
    }
}

/// Parse an exact completion list.
fn parse_completion_entries(content: &str) -> Result<Vec<ExpectedCompletion>, String> {
    let mut expected = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some(completion) = parse_completion_entry(line) else {
            return Err(format!(
                "completion expectation line '{line}' is invalid: expected 'label: kind'"
            ));
        };

        expected.push(completion);
    }

    Ok(expected)
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
