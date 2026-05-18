use std::path::{Path, PathBuf};

use destack_query as query;
use destack_source::Uri;

use crate::core::CaseResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a file rename test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    // handle missing expectation
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    CaseResult::Skipped {
        reason: "no file_rename expectation provided".to_string(),
    }
}

/// Parse a file rename expectation into rename entries.
pub(crate) fn parse_rename_entries(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
) -> Result<Vec<query::FileRenameEntry>, String> {
    // collect path tokens
    let mut tokens = Vec::new();
    tokens.push(expectation.target.as_str());
    tokens.extend(expectation.args.iter().map(|arg| arg.as_str()));

    // validate input pairs
    if tokens.len() < 2 {
        return Err("file_rename requires <old> <new> path pairs".to_string());
    }
    if tokens.len() % 2 != 0 {
        return Err("file_rename expects an even number of path arguments".to_string());
    }

    // resolve paths relative to the workspace root
    let root = &session.root;
    let mut entries = Vec::new();
    for pair in tokens.chunks(2) {
        let old_path = resolve_rename_path(root, pair[0])?;
        let new_path = resolve_rename_path(root, pair[1])?;
        entries.push(query::FileRenameEntry { old_path, new_path });
    }

    Ok(entries)
}

/// Run with markdown expectation.
///
/// Format: `query file_rename old_path new_path [old_path new_path ...]`.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // parse rename entries
    let renames = match parse_rename_entries(session, exp) {
        Ok(renames) => renames,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    // execute rename files query
    let workspace = session.workspace_context();
    let result = query::rename_files(&workspace, &renames);

    // allow explicit failure expectations
    let content = exp.content.trim();
    if content == "<none>" {
        return match result {
            None => CaseResult::Passed,
            Some(rename_result) => CaseResult::Failed {
                message: format!(
                    "file_rename should have produced no edits but produced {}",
                    rename_result.total_edits()
                ),
            },
        };
    }

    // require a result when expecting edits
    let Some(rename_result) = result else {
        return CaseResult::Failed {
            message: "file_rename returned no edits".to_string(),
        };
    };

    // empty expectation means any edits are acceptable
    if content.is_empty() {
        // reject empty edit sets
        if rename_result.total_edits() == 0 {
            return CaseResult::Failed {
                message: "file_rename produced 0 edits".to_string(),
            };
        }

        return CaseResult::Passed;
    }

    // parse expected edit count
    let Ok(expected_count) = content.parse::<usize>() else {
        return CaseResult::Failed {
            message: format!("file_rename expectation '{content}' is not a valid count"),
        };
    };

    // ensure expected edit count matches
    if rename_result.total_edits() != expected_count {
        return CaseResult::Failed {
            message: format!(
                "file_rename produced {} edits, expected {}",
                rename_result.total_edits(),
                expected_count
            ),
        };
    }

    CaseResult::Passed
}

/// Resolve a rename path from input.
fn resolve_rename_path(root: &Path, raw: &str) -> Result<PathBuf, String> {
    // resolve file uris
    if raw.starts_with("file://") {
        let uri = Uri::from_string(raw);
        let Some(path) = uri.to_path_buf() else {
            return Err(format!("failed to resolve file uri '{raw}'"));
        };

        return Ok(path);
    }

    // resolve absolute and workspace relative paths
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(root.join(path))
}
