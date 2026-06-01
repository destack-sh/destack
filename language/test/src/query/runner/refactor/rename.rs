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
    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    if content == "<none>" {
        let result = ctx.rename(&workspace, offset, new_name);
        return match result {
            None => CaseResult::Passed,
            Some(rename_result) => CaseResult::Failed {
                message: format!(
                    "rename at '{}' to '{}' should have failed but produced {} edits",
                    exp.target,
                    new_name,
                    rename_result.total_edits(),
                ),
            },
        };
    }

    // first check prepare_rename
    let prepare_result = ctx.rename_target(offset);
    if prepare_result.is_none() {
        return CaseResult::Failed {
            message: format!("prepare_rename at '{}' returned None", exp.target),
        };
    }

    // then do the actual rename
    let result = ctx.rename(&workspace, offset, new_name);

    // empty expectation means we just verify the rename works (produces any edits)
    if content.is_empty() {
        match result {
            Some(rename_result) => {
                if rename_result.total_edits() == 0 {
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
            if rename_result.total_edits() != expected_count {
                CaseResult::Failed {
                    message: format!(
                        "rename at '{}' to '{}' produced {} edits, expected {}",
                        exp.target,
                        new_name,
                        rename_result.total_edits(),
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
