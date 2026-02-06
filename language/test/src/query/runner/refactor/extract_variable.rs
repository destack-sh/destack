use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_span;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an extract variable test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no extract_variable expectation provided".to_string(),
    }
}

/// Run an extract variable test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve the selection span
    let selection = match resolve_query_span(session, &exp.target) {
        Ok(span) => span,
        Err(error) => return TestResult::Failed { message: error },
    };

    let new_name = exp
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("extracted");

    let result = query::extract_variable(&session.session, session.file_id, selection, new_name);

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(result) => TestResult::Failed {
                message: format!(
                    "extract_variable should have produced no edits but produced {}",
                    result.edits.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return TestResult::Failed {
            message: "extract_variable returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.edits.total_edits() == 0 {
            return TestResult::Failed {
                message: "extract_variable produced 0 edits".to_string(),
            };
        }

        return TestResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("extract_variable expectation '{content}' is not a valid count"),
        };
    };

    if result.edits.total_edits() != expected_count {
        return TestResult::Failed {
            message: format!(
                "extract_variable produced {} edits, expected {}",
                result.edits.total_edits(),
                expected_count,
            ),
        };
    }

    TestResult::Passed
}
