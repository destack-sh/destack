use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a selection_range test.
///
/// Tests that selecting at a position produces the expected depth of nested ranges.
/// Format: `query selection_range $0` with content showing expected depth.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    TestResult::Skipped {
        reason: "no selection_range expectation provided".to_string(),
    }
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // parse cursor from target (e.g., "$0" or marker name)
    let offset = if exp.target.starts_with('$') {
        let cursor_idx: usize = exp.target[1..].parse().unwrap_or(0);
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };
        cursor.offset
    } else {
        let Some(marker) = session.markers.range(&exp.target) else {
            return TestResult::Failed {
                message: format!("marker '{}' not found", exp.target),
            };
        };
        marker.span.start
    };

    let ranges = query::selection_ranges(&session.session, session.file_id, &[offset]);

    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let depth = ranges.first().map(|r| r.depth()).unwrap_or(0);
        return TestResult::Failed {
            message: format!(
                "selection_range expectation is empty, got depth {}",
                depth
            ),
        };
    }

    // parse expected depth from content
    let Ok(expected_depth) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("selection_range expectation '{content}' is not a valid depth"),
        };
    };

    let actual_depth = ranges.first().map(|r| r.depth()).unwrap_or(0);

    if actual_depth != expected_depth {
        TestResult::Failed {
            message: format!(
                "selection_range at '{}' has depth {actual_depth}, expected {expected_depth}",
                exp.target
            ),
        }
    } else {
        TestResult::Passed
    }
}
