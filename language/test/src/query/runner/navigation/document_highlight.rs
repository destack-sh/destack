use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_highlight test.
///
/// Tests that querying from a marker highlights the expected ranges.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check highlight expectations from markers
    for (cursor_idx, expected_highlights) in &session.markers.expectations.highlights {
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };

        let highlights = query::document_highlight(&session.session, session.file_id, cursor.offset);

        if highlights.len() != expected_highlights.len() {
            return TestResult::Failed {
                message: format!(
                    "document_highlight at ${} returned {} highlights, expected {}",
                    cursor_idx,
                    highlights.len(),
                    expected_highlights.len()
                ),
            };
        }

        for exp in expected_highlights {
            let found = highlights.iter().any(|h| {
                h.range.start == exp.start && h.range.end == exp.end
            });

            if !found {
                return TestResult::Failed {
                    message: format!(
                        "highlight {}-{} not found at ${}",
                        exp.start, exp.end, cursor_idx
                    ),
                };
            }
        }
    }

    TestResult::Passed
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

    let highlights = query::document_highlight(&session.session, session.file_id, offset);

    // parse expected count from content
    let expected_count: usize = exp.content.trim().parse().unwrap_or(0);

    if expected_count > 0 && highlights.len() != expected_count {
        TestResult::Failed {
            message: format!(
                "document_highlight at '{}' returned {} highlights, expected {}",
                exp.target,
                highlights.len(),
                expected_count
            ),
        }
    } else {
        TestResult::Passed
    }
}
