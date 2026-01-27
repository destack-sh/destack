use destack_source::Span;
use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_implementation test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no goto_implementation expectation provided".to_string(),
        };
    };

    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // run the query
    let result = query::goto_implementation(&session.session, file_id, offset);

    let Some(result) = result else {
        return TestResult::Failed {
            message: "goto_implementation returned None".to_string(),
        };
    };

    let expected_content = exp.content.trim();

    // empty expectations are not allowed
    if expected_content.is_empty() {
        return TestResult::Failed {
            message: format!(
                "goto_implementation expectation is empty, got {} locations",
                result.locations.len()
            ),
        };
    }

    // allow explicit empty expectations
    if expected_content == "<none>" {
        return if result.locations.is_empty() {
            TestResult::Passed
        } else {
            TestResult::Failed {
                message: format!(
                    "goto_implementation expected no locations, got {}",
                    result.locations.len(),
                ),
            }
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_implementation_invariants(session, &result.locations) {
        return TestResult::Failed { message };
    }

    // compare against a protocol shaped snapshot when structured
    if is_snapshot_expectation(expected_content) {
        let actual_snapshot = result
            .locations
            .iter()
            .map(|span| format_span_for_session(session, *span))
            .collect::<Vec<_>>()
            .join("\n");
        let actual_snapshot = normalize_expected_snapshot(&actual_snapshot);
        let expected_snapshot = normalize_expected_snapshot(expected_content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "goto_implementation snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
                ),
            };
        }

        return TestResult::Passed;
    }

    // parse expected count from expectation content
    let Ok(expected_count) = expected_content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!(
                "goto_implementation expectation '{expected_content}' is not a valid count"
            ),
        };
    };

    if result.locations.len() != expected_count {
        return TestResult::Failed {
            message: format!(
                "expected {} implementations, found {}",
                expected_count,
                result.locations.len()
            ),
        };
    }

    TestResult::Passed
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by span markers
    expected.lines().map(str::trim).any(|line| {
        let has_span = line.contains(':') && line.chars().any(|ch| ch.is_ascii_digit());
        let has_range = line.contains("range=");
        has_span || has_range
    })
}

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
    let mut previous: Option<(u32, u32, u32)> = None;
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
