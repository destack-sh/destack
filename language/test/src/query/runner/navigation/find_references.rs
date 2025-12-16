use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a find_references test.
///
/// Tests that querying from a marker finds the expected number of references.
/// Format: `query find_references def:foo` with content showing expected count or markers.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check reference_count expectations from markers
    for (marker_name, expected_count) in &session.markers.expectations.reference_count {
        let Some(marker) = session.markers.range(marker_name) else {
            return TestResult::Failed {
                message: format!("marker '{marker_name}' not found"),
            };
        };

        let offset = marker.span.start;
        let result = query::find_references(&session.session, session.file_id, offset, true);

        match result {
            Some(refs) => {
                if refs.len() != *expected_count {
                    return TestResult::Failed {
                        message: format!(
                            "find_references at '{marker_name}' returned {} references, expected {expected_count}",
                            refs.len(),
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("find_references at '{marker_name}' returned None",),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let Some(source_marker) = session.markers.range(&exp.target) else {
        return TestResult::Failed {
            message: format!("source marker '{}' not found", exp.target),
        };
    };

    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let offset = source_marker.span.start;
        let result = query::find_references(&session.session, session.file_id, offset, true);
        return TestResult::Failed {
            message: format!(
                "find_references expectation is empty at '{}', got: {:?}",
                exp.target,
                result.map(|r| r.len())
            ),
        };
    }

    // parse expected count from content (e.g., "3" or "count: 3")
    let count_str = content.strip_prefix("count:").unwrap_or(content).trim();
    let Ok(expected_count) = count_str.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("find_references expectation '{content}' is not a valid count",),
        };
    };

    let offset = source_marker.span.start;
    let result = query::find_references(&session.session, session.file_id, offset, true);

    match result {
        Some(refs) => {
            if refs.len() != expected_count {
                TestResult::Failed {
                    message: format!(
                        "find_references at '{}' returned {} references, expected {expected_count}",
                        exp.target,
                        refs.len(),
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("find_references at '{}' returned None", exp.target),
        },
    }
}
