use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an inline refactor test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no inline expectation provided".to_string(),
    }
}

/// Run an inline refactor test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve target position
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(pos) => pos,
        Err(error) => return CaseResult::Failed { message: error },
    };

    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let result = ctx.inline_symbol(&workspace, offset);

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => CaseResult::Passed,
            Some(result) => CaseResult::Failed {
                message: format!(
                    "inline should have produced no edits but produced {}",
                    result.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return CaseResult::Failed {
            message: "inline returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.total_edits() == 0 {
            return CaseResult::Failed {
                message: "inline produced 0 edits".to_string(),
            };
        }

        return CaseResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return CaseResult::Failed {
            message: format!("inline expectation '{content}' is not a valid count"),
        };
    };

    if result.total_edits() != expected_count {
        return CaseResult::Failed {
            message: format!(
                "inline produced {} edits, expected {}",
                result.total_edits(),
                expected_count
            ),
        };
    }

    CaseResult::Passed
}
