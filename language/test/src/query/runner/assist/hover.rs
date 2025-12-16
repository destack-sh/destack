use destack_workspace::query;

use crate::harness::TestResult;
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
                    message: format!("hover at ${} returned None", cursor_idx),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // parse cursor from target (e.g., "$0" or marker name)
    let offset = if exp.target.starts_with('$') {
        let cursor_idx: usize = exp.target[1..].parse().unwrap_or(0);
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };
        cursor.offset
    } else {
        let Some(marker) = session.markers.range(&exp.target) else {
            return TestResult::Failed {
                message: format!("marker '{}' not found", exp.target),
            };
        };
        marker.span.start
    };

    let result = query::hover(&session.session, session.file_id, offset);

    let expected_text = exp.content.trim();

    match result {
        Some(hover_info) => {
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
        None => {
            if expected_text.is_empty() || expected_text == "none" {
                TestResult::Passed
            } else {
                TestResult::Failed {
                    message: format!("hover at '{}' returned None", exp.target),
                }
            }
        }
    }
}
