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
                            "find_references at '{}' returned {} references, expected {}",
                            marker_name,
                            refs.len(),
                            expected_count
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("find_references at '{}' returned None", marker_name),
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

    let offset = source_marker.span.start;
    let result = query::find_references(&session.session, session.file_id, offset, true);

    // parse expected count from content (e.g., "3" or "count: 3")
    let expected_count: usize = exp
        .content
        .trim()
        .strip_prefix("count:")
        .unwrap_or(exp.content.trim())
        .trim()
        .parse()
        .unwrap_or(0);

    match result {
        Some(refs) => {
            if expected_count > 0 && refs.len() != expected_count {
                TestResult::Failed {
                    message: format!(
                        "find_references at '{}' returned {} references, expected {}",
                        exp.target,
                        refs.len(),
                        expected_count
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => {
            if expected_count == 0 {
                TestResult::Passed
            } else {
                TestResult::Failed {
                    message: format!("find_references at '{}' returned None", exp.target),
                }
            }
        }
    }
}
