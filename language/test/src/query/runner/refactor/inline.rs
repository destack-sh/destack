use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an inline refactor test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no inline expectation provided".to_string(),
    }
}

/// Run an inline refactor test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve target position
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(pos) => pos,
        Err(error) => return TestResult::Failed { message: error },
    };

    let result = query::inline_symbol(&session.session, file_id, offset);

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(result) => TestResult::Failed {
                message: format!(
                    "inline should have produced no edits but produced {}",
                    result.edits.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return TestResult::Failed {
            message: "inline returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.edits.total_edits() == 0 {
            return TestResult::Failed {
                message: "inline produced 0 edits".to_string(),
            };
        }

        return TestResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("inline expectation '{content}' is not a valid count"),
        };
    };

    if result.edits.total_edits() != expected_count {
        return TestResult::Failed {
            message: format!(
                "inline produced {} edits, expected {}",
                result.edits.total_edits(),
                expected_count
            ),
        };
    }

    TestResult::Passed
}
