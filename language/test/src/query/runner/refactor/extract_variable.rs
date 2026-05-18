use destack_query as query;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_span;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an extract variable test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no extract_variable expectation provided".to_string(),
    }
}

/// Run an extract variable test with an expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the selection span
    let selection = match resolve_query_span(session, &exp.target) {
        Ok(span) => span,
        Err(error) => return CaseResult::Failed { message: error },
    };

    let new_name = exp
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("extracted");

    let ctx = session.module_context(selection.file);
    let result = query::extract_variable(&ctx, selection, new_name);

    // allow explicit no-edit expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => CaseResult::Passed,
            Some(result) => CaseResult::Failed {
                message: format!(
                    "extract_variable should have produced no edits but produced {}",
                    result.total_edits()
                ),
            },
        };
    }

    let Some(result) = result else {
        return CaseResult::Failed {
            message: "extract_variable returned no edits".to_string(),
        };
    };

    if content.is_empty() {
        if result.total_edits() == 0 {
            return CaseResult::Failed {
                message: "extract_variable produced 0 edits".to_string(),
            };
        }

        return CaseResult::Passed;
    }

    let Ok(expected_count) = content.parse::<usize>() else {
        return CaseResult::Failed {
            message: format!("extract_variable expectation '{content}' is not a valid count"),
        };
    };

    if result.total_edits() != expected_count {
        return CaseResult::Failed {
            message: format!(
                "extract_variable produced {} edits, expected {}",
                result.total_edits(),
                expected_count,
            ),
        };
    }

    CaseResult::Passed
}
