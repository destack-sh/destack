use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a prepare_rename test.
///
/// Verifies that prepare_rename returns a valid range and placeholder for renameable symbols.
/// Format: `query prepare_rename def:foo` with content showing expected placeholder name.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no prepare_rename expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // parse marker from target
    let Some(marker) = session.markers.range(&exp.target) else {
        return TestResult::Failed {
            message: format!("marker '{}' not found", exp.target),
        };
    };

    // resolve target file and offset
    let file_id = marker.span.file;
    let offset = marker.span.start;

    // run prepare rename at the resolved position
    let result = query::prepare_rename(&session.session, file_id, offset);

    let content = exp.content.trim();

    // check for explicit "cannot rename" expectation
    if content == "<none>" {
        return match result {
            Some(prepare_result) => TestResult::Failed {
                message: format!(
                    "prepare_rename at '{}' should have failed but got placeholder '{}'",
                    exp.target, prepare_result.placeholder
                ),
            },
            None => TestResult::Passed,
        };
    }

    // empty expectation just checks that prepare_rename succeeds
    if content.is_empty() {
        return match result {
            Some(_) => TestResult::Passed,
            None => TestResult::Failed {
                message: format!("prepare_rename at '{}' returned None", exp.target),
            },
        };
    }

    // compare the placeholder against the expected name
    match result {
        Some(prepare_result) => {
            // check for placeholder mismatch
            if prepare_result.placeholder != content {
                TestResult::Failed {
                    message: format!(
                        "prepare_rename at '{}' returned placeholder '{}', expected '{}'",
                        exp.target, prepare_result.placeholder, content
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("prepare_rename at '{}' returned None", exp.target),
        },
    }
}
