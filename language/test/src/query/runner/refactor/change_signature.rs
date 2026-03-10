use destack_query as query;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a change signature test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no change_signature expectation provided".to_string(),
    }
}

/// Run a change signature test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve target position
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(pos) => pos,
        Err(error) => return TestResult::Failed { message: error },
    };

    let new_parameters = exp.args.first().map(|value| value.as_str()).unwrap_or("");
    let new_arguments = exp.args.get(1).map(|value| value.as_str()).unwrap_or("");

    let result = query::change_signature(
        &session.session,
        file_id,
        offset,
        new_parameters,
        new_arguments,
    );

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(result) => TestResult::Failed {
                message: format!(
                    "change_signature should have produced no edits but produced {}",
                    result.edits.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return TestResult::Failed {
            message: "change_signature returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.edits.total_edits() == 0 {
            return TestResult::Failed {
                message: "change_signature produced 0 edits".to_string(),
            };
        }

        return TestResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("change_signature expectation '{content}' is not a valid count"),
        };
    };

    if result.edits.total_edits() != expected_count {
        return TestResult::Failed {
            message: format!(
                "change_signature produced {} edits, expected {}",
                result.edits.total_edits(),
                expected_count
            ),
        };
    }

    TestResult::Passed
}
