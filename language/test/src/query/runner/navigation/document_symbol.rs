use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_symbols test.
///
/// Tests that document symbols returns the expected symbols for a file.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no document_symbols expectation".to_string(),
        };
    };

    let expected = exp.content.trim();

    // empty expectation is an error - must specify expected symbols
    if expected.is_empty() {
        let symbols = query::document_symbols(&session.session, session.file_id);
        let actual_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return TestResult::Failed {
            message: format!(
                "document_symbols expectation is empty, but query returned: {actual_names:?}"
            ),
        };
    }

    let symbols = query::document_symbols(&session.session, session.file_id);
    let actual_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    // check if we should verify count
    if let Some(count_str) = expected.strip_prefix("count:") {
        let expected_count: usize = count_str.trim().parse().unwrap_or(0);
        if symbols.len() != expected_count {
            return TestResult::Failed {
                message: format!(
                    "document_symbols returned {} symbols, expected {}",
                    symbols.len(),
                    expected_count
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

    for name in &expected_names {
        if !actual_names.contains(name) {
            return TestResult::Failed {
                message: format!(
                    "document_symbols missing expected symbol '{name}', got: '{actual_names:?}'",
                ),
            };
        }
    }

    TestResult::Passed
}
