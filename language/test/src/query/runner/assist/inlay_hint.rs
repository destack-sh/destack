use destack_source::Span;
use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run an inlay_hints test.
///
/// Verifies that inlay hints are shown at expected positions.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check inlay_hint expectations from markers
    let expected_hints = &session.markers.expectations.inlay_hints;
    if expected_hints.is_empty() {
        return TestResult::Skipped {
            reason: "no inlay_hint expectations defined".to_string(),
        };
    }

    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let hints = query::inlay_hints(&session.session, session.file_id, range);

    for exp in expected_hints {
        let found = hints.iter().any(|h| h.position == exp.offset && h.label.contains(&exp.label));

        if !found {
            let actual: Vec<_> = hints.iter().map(|h| format!("{}:{}", h.position, h.label)).collect();
            return TestResult::Failed {
                message: format!(
                    "inlay_hint at offset {} with label '{}' not found\nactual: {:?}",
                    exp.offset, exp.label, actual
                ),
            };
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let hints = query::inlay_hints(&session.session, session.file_id, range);

    // parse expected count from content
    let expected_count: usize = exp.content.trim().parse().unwrap_or(0);

    if expected_count > 0 && hints.len() != expected_count {
        TestResult::Failed {
            message: format!(
                "inlay_hints returned {} hints, expected {}",
                hints.len(),
                expected_count
            ),
        }
    } else {
        TestResult::Passed
    }
}
