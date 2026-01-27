use destack_source::Span;
use destack_workspace::query;
use destack_workspace::query::HoverInfo;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a hover test.
///
/// Verifies that hover at cursor position shows expected information.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check hover expectations from markers
    for (cursor_idx, expected_text) in &session.markers.expectations.hover {
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };

        let result = query::hover(&session.session, session.file_id, cursor.offset);

        match result {
            Some(hover_info) => {
                // validate hover invariants before substring checks
                if let Err(message) = validate_hover_invariants(session, &hover_info, cursor.offset)
                {
                    return TestResult::Failed { message };
                }

                if !hover_info.signature.contains(expected_text) {
                    return TestResult::Failed {
                        message: format!(
                            "hover at ${} expected to contain '{}', got '{}'",
                            cursor_idx, expected_text, hover_info.signature
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("hover at ${cursor_idx} returned None"),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // run the hover query once
    let result = query::hover(&session.session, file_id, offset);

    // normalize the expected content
    let expected_text = exp.content.trim();

    // empty expectation is an error
    if expected_text.is_empty() {
        return TestResult::Failed {
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
            None => TestResult::Passed,
            Some(hover_info) => TestResult::Failed {
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
                return TestResult::Failed { message };
            }

            // compare against protocol shaped snapshots when structured
            if is_snapshot_expectation(expected_text) {
                let actual_snapshot =
                    normalize_expected_snapshot(&format_hover_snapshot(session, &hover_info));
                let expected_snapshot = normalize_expected_snapshot(expected_text);

                if actual_snapshot != expected_snapshot {
                    return TestResult::Failed {
                        message: format!(
                            "hover snapshot mismatch at '{}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}",
                            exp.target
                        ),
                    };
                }

                return TestResult::Passed;
            }

            // fall back to substring matching for legacy expectations
            if !hover_info.signature.contains(expected_text) {
                TestResult::Failed {
                    message: format!(
                        "hover at '{}' expected to contain '{}', got '{}'",
                        exp.target, expected_text, hover_info.signature
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("hover at '{}' returned None", exp.target),
        },
    }
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by signature or range markers
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains("signature=") || line.contains("range="))
}

/// Validate hover invariants.
fn validate_hover_invariants(
    session: &QueryTestSession,
    hover: &HoverInfo,
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
                "hover range {:?} does not contain offset {}",
                range, offset
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
fn format_hover_snapshot(session: &QueryTestSession, hover: &HoverInfo) -> String {
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
