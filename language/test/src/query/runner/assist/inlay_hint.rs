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

    for expected_hint in expected_hints {
        let found = hints
            .iter()
            .any(|h| h.position == expected_hint.offset && h.label.contains(&expected_hint.label));

        if !found {
            let actual: Vec<_> = hints
                .iter()
                .map(|h| format!("{}:{}", h.position, h.label))
                .collect();
            return TestResult::Failed {
                message: format!(
                    "inlay_hint at offset {} with label '{}' not found\nactual: {:?}",
                    expected_hint.offset, expected_hint.label, actual
                ),
            };
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let range = Span::new(session.file_id, 0, session.source.len() as u32);
        let hints = query::inlay_hints(&session.session, session.file_id, range);
        return TestResult::Failed {
            message: format!(
                "inlay_hints expectation is empty, but query returned {} hints",
                hints.len()
            ),
        };
    }

    // parse expected count from content
    let Ok(expected_count) = content.parse::<usize>() else {
        return TestResult::Failed {
            message: format!("inlay_hints expectation '{content}' is not a valid count"),
        };
    };

    // get all hints for the file
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let hints = query::inlay_hints(&session.session, session.file_id, range);

    if hints.len() != expected_count {
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
