use destack_query as query;
use destack_query::Hover;
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::expectation::{
    TextExpectation, describe_text_expectation, matches_text_expectation,
};
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a hover test.
///
/// Verifies that hover at cursor position shows expected information.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no hover expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // run the hover query once
    let ctx = session.module_context(file_id);
    let result = query::hover(&ctx, offset);

    // normalize the expected content
    let expected_text = exp.content.trim();

    // empty expectation is an error
    if expected_text.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "hover expectation is empty at '{}', got: {:?}",
                exp.target,
                result.map(|r| r.signature)
            ),
        };
    }

    // "<none>" means we expect no result
    if expected_text == "<none>" {
        return match result {
            None => CaseResult::Passed,
            Some(hover_info) => CaseResult::Failed {
                message: format!(
                    "hover at '{}' expected None, got '{}'",
                    exp.target, hover_info.signature
                ),
            },
        };
    }

    // require a hover result for non-empty expectations
    match result {
        Some(hover_info) => {
            // validate hover invariants before comparisons
            if let Err(message) = validate_hover_invariants(session, &hover_info, offset) {
                return CaseResult::Failed { message };
            }

            // compare against protocol shaped snapshots when structured
            if looks_like_snapshot(expected_text, &["signature=", "range="]) {
                let actual_snapshot = format_hover_snapshot(session, &hover_info);
                return compare_snapshot(
                    &format!("hover at '{}'", exp.target),
                    &actual_snapshot,
                    expected_text,
                );
            }

            // explicit query blocks are exact only
            let expectation = match TextExpectation::parse(expected_text) {
                Ok(expectation) => expectation,
                Err(message) => return CaseResult::Failed { message },
            };
            if !matches_text_expectation(&hover_info.signature, expectation) {
                CaseResult::Failed {
                    message: format!(
                        "hover at '{}' expected to {} '{}', got '{}'",
                        exp.target,
                        describe_text_expectation(expectation),
                        expectation.text(),
                        hover_info.signature
                    ),
                }
            } else {
                CaseResult::Passed
            }
        }
        None => CaseResult::Failed {
            message: format!("hover at '{}' returned None", exp.target),
        },
    }
}

/// Validate hover invariants.
fn validate_hover_invariants(
    session: &QueryTestSession,
    hover: &Hover,
    offset: u32,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let source = source_for_file(session, session.file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // validate the hover signature
    let signature_is_empty = hover.signature.trim().is_empty();
    if signature_is_empty {
        errors.push("hover signature is empty".to_string());
    }

    // validate the hover range when present
    if let Some(range) = hover.range {
        validate_span_bounds("hover range", range, source_len, &mut errors);

        // allow hover at the end of a symbol by checking the previous byte
        let contains_offset =
            range.contains(offset) || (offset > 0 && range.contains(offset.saturating_sub(1)));

        // reject ranges that do not cover the hover position
        if !contains_offset {
            errors.push(format!(
                "hover range {range:?} does not contain offset {offset}"
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "hover invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Validate that a span is within the source bounds.
fn validate_span_bounds(label: &str, span: Span, source_len: u32, errors: &mut Vec<String>) {
    // ensure the span start does not exceed the end
    if span.start > span.end {
        errors.push(format!(
            "{label}: span start {} is after end {}",
            span.start, span.end
        ));
    }

    // ensure the span end stays within the source bounds
    if span.end > source_len {
        errors.push(format!(
            "{label}: span end {} exceeds source length {}",
            span.end, source_len
        ));
    }
}

/// Format hover info as a protocol shaped snapshot.
fn format_hover_snapshot(session: &QueryTestSession, hover: &Hover) -> String {
    // render the hover range as a span when available
    let range_text = hover
        .range
        .map(|range| format_span_for_session(session, range))
        .unwrap_or("<none>".to_string());

    // render documentation as a single line
    let documentation = hover
        .documentation
        .as_deref()
        .map(flatten_text)
        .unwrap_or_else(|| "<none>".to_string());

    let signature = flatten_text(&hover.signature);

    format!("range={range_text} signature={signature} documentation={documentation}")
}

/// Flatten multi-line text into a single line for snapshots.
fn flatten_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
