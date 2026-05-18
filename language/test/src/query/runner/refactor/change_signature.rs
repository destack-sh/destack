use destack_query as query;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a change signature test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no change_signature expectation provided".to_string(),
    }
}

/// Run a change signature test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve target position
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(pos) => pos,
        Err(error) => return CaseResult::Failed { message: error },
    };

    let new_parameters = exp.args.first().map(|value| value.as_str()).unwrap_or("");
    let new_arguments = exp.args.get(1).map(|value| value.as_str()).unwrap_or("");

    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let result = query::change_signature(&ctx, &workspace, offset, new_parameters, new_arguments);

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => CaseResult::Passed,
            Some(result) => CaseResult::Failed {
                message: format!(
                    "change_signature should have produced no edits but produced {}",
                    result.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return CaseResult::Failed {
            message: "change_signature returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.total_edits() == 0 {
            return CaseResult::Failed {
                message: "change_signature produced 0 edits".to_string(),
            };
        }

        return CaseResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return CaseResult::Failed {
            message: format!("change_signature expectation '{content}' is not a valid count"),
        };
    };

    if result.total_edits() != expected_count {
        return CaseResult::Failed {
            message: format!(
                "change_signature produced {} edits, expected {}",
                result.total_edits(),
                expected_count
            ),
        };
    }

    CaseResult::Passed
}
