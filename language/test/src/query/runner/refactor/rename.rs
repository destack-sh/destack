use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a rename test.
///
/// Verifies that renaming a symbol produces the expected edits.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no rename expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
///
/// Format: `query rename def:foo "newName"` with content showing expected edit count.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // parse marker from target
    let Some(marker) = session.markers.range(&exp.target) else {
        return TestResult::Failed {
            message: format!("marker '{}' not found", exp.target),
        };
    };

    // get new name from args
    let new_name = exp.args.first().map(|s| s.as_str()).unwrap_or("newName");

    let offset = marker.span.start;

    // first check prepare_rename
    let prepare_result = query::prepare_rename(&session.session, session.file_id, offset);
    if prepare_result.is_none() {
        return TestResult::Failed {
            message: format!("prepare_rename at '{}' returned None", exp.target),
        };
    }

    // then do the actual rename
    let result = query::rename(&session.session, session.file_id, offset, new_name);

    let content = exp.content.trim();

    // empty expectation means we just verify the rename works (produces any edits)
    if content.is_empty() {
        match result {
            Some(rename_result) => {
                if rename_result.edit_count() == 0 {
                    return TestResult::Failed {
                        message: format!(
                            "rename at '{}' to '{}' produced 0 edits",
                            exp.target, new_name
                        ),
                    };
                }
                return TestResult::Passed;
            }
            None => {
                return TestResult::Failed {
                    message: format!("rename at '{}' to '{}' returned None", exp.target, new_name),
                };
            }
        }
    }

    // parse expected edit count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("rename expectation '{}' is not a valid count", content),
        };
    };

    match result {
        Some(rename_result) => {
            if rename_result.edit_count() != expected_count {
                TestResult::Failed {
                    message: format!(
                        "rename at '{}' to '{}' produced {} edits, expected {}",
                        exp.target,
                        new_name,
                        rename_result.edit_count(),
                        expected_count
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("rename at '{}' to '{}' returned None", exp.target, new_name),
        },
    }
}
