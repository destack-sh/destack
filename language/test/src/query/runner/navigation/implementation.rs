use destack_query as query;
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_span_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_implementation test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no goto_implementation expectation provided".to_string(),
        };
    };

    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // run the query
    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let targets = query::goto_implementation(&ctx, &workspace, offset);
    let locations = targets
        .iter()
        .map(|target| target.target.span)
        .collect::<Vec<_>>();

    let expected_content = exp.content.trim();

    // empty expectations are not allowed
    if expected_content.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "goto_implementation expectation is empty, got {} locations",
                locations.len()
            ),
        };
    }

    // allow explicit empty expectations
    if expected_content == "<none>" {
        return if locations.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "goto_implementation expected no locations, got {}",
                    locations.len(),
                ),
            }
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_implementation_invariants(session, &locations) {
        return CaseResult::Failed { message };
    }

    // compare against a protocol shaped snapshot when structured
    if looks_like_span_snapshot(expected_content, &["range="]) {
        let actual_snapshot = locations
            .iter()
            .map(|span| format_span_for_session(session, *span))
            .collect::<Vec<_>>()
            .join("\n");
        return compare_snapshot("goto_implementation", &actual_snapshot, expected_content);
    }

    CaseResult::Failed {
        message: format!(
            "goto_implementation requires an explicit location snapshot, got '{expected_content}'"
        ),
    }
}

/// Decide whether an expectation is a structured snapshot.
/// Validate implementation location invariants.
fn validate_implementation_invariants(
    session: &QueryTestSession,
    locations: &[Span],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate span bounds for each location
    for span in locations {
        let source = source_for_file(session, span.file);
        let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

        // ensure the span start does not exceed the end
        if span.start > span.end {
            errors.push(format!(
                "implementation span start {} is after end {}",
                span.start, span.end
            ));
        }

        // ensure the span end stays within the source bounds
        if span.end > source_len {
            errors.push(format!(
                "implementation span end {} exceeds source length {}",
                span.end, source_len
            ));
        }
    }

    // validate ordering and duplicates
    let mut previous: Option<(u128, u32, u32)> = None;
    for span in locations {
        let key = (span.file.0, span.start, span.end);
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "implementation locations are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("duplicate implementation location {key:?}"));
            }
        }
        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "goto_implementation invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}
