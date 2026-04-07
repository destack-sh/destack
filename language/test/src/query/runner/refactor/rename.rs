use destack_query as query;

use crate::core::CaseResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a rename test.
///
/// Verifies that renaming a symbol produces the expected edits.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no rename expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
///
/// Format: `query rename def:foo "newName"` with content showing expected edit count.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // parse marker from target
    let Some(marker) = session.markers.range(&exp.target) else {
        return CaseResult::Failed {
            message: format!("marker '{}' not found", exp.target),
        };
    };

    // get new name from args
    let new_name = exp.args.first().map(|s| s.as_str()).unwrap_or("newName");

    // resolve target file and offset
    let file_id = marker.span.file;
    let offset = marker.span.start;

    // allow explicit failure expectations
    let content = exp.content.trim();
    if content == "<none>" {
        let result = query::rename(
            &session.repository,
            session.revision,
            file_id,
            offset,
            new_name,
        );
        return match result {
            None => CaseResult::Passed,
            Some(rename_result) => CaseResult::Failed {
                message: format!(
                    "rename at '{}' to '{}' should have failed but produced {} edits",
                    exp.target,
                    new_name,
                    rename_result.edit_count(),
                ),
            },
        };
    }

    // first check prepare_rename
    let prepare_result =
        query::prepare_rename(&session.repository, session.revision, file_id, offset);
    if prepare_result.is_none() {
        return CaseResult::Failed {
            message: format!("prepare_rename at '{}' returned None", exp.target),
        };
    }

    // then do the actual rename
    let result = query::rename(
        &session.repository,
        session.revision,
        file_id,
        offset,
        new_name,
    );

    // empty expectation means we just verify the rename works (produces any edits)
    if content.is_empty() {
        match result {
            Some(rename_result) => {
                if rename_result.edit_count() == 0 {
                    return CaseResult::Failed {
                        message: format!(
                            "rename at '{}' to '{}' produced 0 edits",
                            exp.target, new_name
                        ),
                    };
                }
                return CaseResult::Passed;
            }
            None => {
                return CaseResult::Failed {
                    message: format!("rename at '{}' to '{}' returned None", exp.target, new_name),
                };
            }
        }
    }

    // parse expected edit count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return CaseResult::Failed {
            message: format!("rename expectation '{content}' is not a valid count"),
        };
    };

    match result {
        Some(rename_result) => {
            if rename_result.edit_count() != expected_count {
                CaseResult::Failed {
                    message: format!(
                        "rename at '{}' to '{}' produced {} edits, expected {}",
                        exp.target,
                        new_name,
                        rename_result.edit_count(),
                        expected_count
                    ),
                }
            } else {
                CaseResult::Passed
            }
        }
        None => CaseResult::Failed {
            message: format!("rename at '{}' to '{}' returned None", exp.target, new_name),
        },
    }
}
