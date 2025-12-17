use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a workspace_symbols test.
///
/// Tests that workspace symbol search returns expected symbols.
/// Format: `query workspace_symbols "query"` with content showing expected count or symbol names.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no workspace_symbols expectation".to_string(),
        };
    };

    // the query string is the target (or first arg)
    let query_str = if exp.args.is_empty() {
        &exp.target
    } else {
        &exp.args[0]
    };
    let query_str = query_str.trim_matches('"');

    let symbols = query::workspace_symbols(&session.session, query_str, 1000);

    let expected = exp.content.trim();

    // empty expectation is an error
    if expected.is_empty() {
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return TestResult::Failed {
            message: format!(
                "workspace_symbols expectation is empty, query '{query_str}' returned: {names:?}"
            ),
        };
    }

    // check if we should verify count
    if let Ok(expected_count) = expected.parse::<usize>() {
        if symbols.len() != expected_count {
            let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
            return TestResult::Failed {
                message: format!(
                    "workspace_symbols for '{query_str}' returned {} symbols ({names:?}), expected {expected_count}",
                    symbols.len()
                ),
            };
        }
        return TestResult::Passed;
    }

    // check expected symbol names
    let expected_names: Vec<&str> = expected
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let actual_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    for name in &expected_names {
        if !actual_names.contains(name) {
            return TestResult::Failed {
                message: format!(
                    "workspace_symbols for '{query_str}' missing '{name}', got: {actual_names:?}"
                ),
            };
        }
    }

    TestResult::Passed
}
