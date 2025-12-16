use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a folding_ranges test.
///
/// Tests that the file produces the expected number of folding ranges.
/// Format: `query folding_ranges` with content showing expected count.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no folding_ranges expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let ranges = query::folding_ranges(&session.session, session.file_id);

    let content = exp.content.trim();

    // empty expectation is an error - must specify expected count
    if content.is_empty() {
        return TestResult::Failed {
            message: format!(
                "folding_ranges expectation is empty, got {} ranges",
                ranges.len()
            ),
        };
    }

    // parse expected count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("folding_ranges expectation '{content}' is not a valid count"),
        };
    };

    if ranges.len() != expected_count {
        TestResult::Failed {
            message: format!(
                "folding_ranges returned {} ranges, expected {expected_count}",
                ranges.len(),
            ),
        }
    } else {
        TestResult::Passed
    }
}
