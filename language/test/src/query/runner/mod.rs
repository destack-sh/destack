pub mod assist;
pub mod diagnostic;
pub mod navigation;
pub mod refactor;

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use crate::harness::{TestResult, fixtures_dir};
use crate::query::{QueryTestSession, test_session};

/// Run all query tests in the fixtures directory.
pub fn run_query_tests() -> Vec<(PathBuf, TestResult)> {
    let query_fixtures = fixtures_dir().join("query");
    let mut results = Vec::new();

    // walk all .ds files in query fixtures
    for category in &["navigation", "refactor", "assist", "diagnostic"] {
        let category_dir = query_fixtures.join(category);
        if !category_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&category_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().map(|e| e == "ds").unwrap_or(false) {
                let result = run_single_test(&path);
                results.push((path, result));
            }
        }
    }

    results
}

/// Run a single query test file.
fn run_single_test(path: &Path) -> TestResult {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    let session = test_session(&source);

    // determine test type from markers
    let test_type = session.markers.test_type.as_deref().unwrap_or("unknown");

    dispatch_test(test_type, &session)
}

/// Dispatch to the appropriate test runner based on test type.
fn dispatch_test(test_type: &str, session: &QueryTestSession) -> TestResult {
    let result = catch_unwind(AssertUnwindSafe(|| {
        match test_type {
            // navigation
            "goto_definition" => navigation::definition::run(session),
            "find_references" => navigation::find_references::run(session),
            "document_highlight" => navigation::document_highlight::run(session),
            "goto_implementation" => navigation::implementation::run(session),
            "document_symbols" => navigation::document_symbol::run(session),
            "selection_range" => navigation::selection_range::run(session),
            "call_hierarchy" => navigation::call_hierarchy::run(session),
            "type_hierarchy" => navigation::type_hierarchy::run(session),
            "document_link" => navigation::document_link::run(session),

            // refactor
            "rename" => refactor::rename::run(session),
            "prepare_rename" => refactor::prepare_rename::run(session),

            // assist
            "completion" => assist::completion::run(session),
            "hover" => assist::hover::run(session),
            "signature_help" => assist::signature_help::run(session),
            "inlay_hints" => assist::inlay_hint::run(session),
            "folding_ranges" => assist::folding::run(session),
            "semantic_tokens" => assist::semantic_token::run(session),
            "code_lens" => assist::code_lens::run(session),

            // diagnostic
            "code_actions" => diagnostic::code_action::run(session),

            _ => TestResult::Skipped {
                reason: format!("unknown test type: {test_type}"),
            },
        }
    }));

    match result {
        Ok(test_result) => test_result,
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("unknown panic");

            // if it's a todo!() panic, skip the test
            if msg.contains("#Incomplete") {
                TestResult::Skipped {
                    reason: msg.to_string(),
                }
            } else {
                TestResult::Failed {
                    message: format!("panic: {msg}"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_query_tests() {
        // just ensure it doesn't panic
        let results = run_query_tests();
        for (path, result) in &results {
            println!("{}: {:?}", path.display(), result);
        }
    }
}
