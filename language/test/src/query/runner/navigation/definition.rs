use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_definition test.
///
/// For each use marker, verify it resolves to the expected def marker.
/// With markdown format, the expectation specifies target marker and expected result.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // if we have a markdown expectation, use it
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: use inline -> target syntax
    for reference in session.markers.references() {
        let Some(target_name) = &reference.target else {
            continue;
        };

        let Some(expected_def) = session.markers.range(target_name) else {
            return TestResult::Failed {
                message: format!("target marker '{target_name}' not found"),
            };
        };

        let offset = reference.span.start;
        let result = query::goto_definition(&session.session, session.file_id, offset);

        match result {
            Some(def_result) => {
                if def_result.locations.is_empty() {
                    return TestResult::Failed {
                        message: format!(
                            "goto_definition at offset {offset} returned empty result"
                        ),
                    };
                }

                let found_match = def_result.locations.iter().any(|loc| {
                    loc.start == expected_def.span.start && loc.end == expected_def.span.end
                });

                if !found_match {
                    return TestResult::Failed {
                        message: format!(
                            "goto_definition at offset {} returned wrong location: expected {:?}, got {:?}",
                            offset, expected_def.span, def_result.locations[0]
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("goto_definition at offset {offset} returned None"),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation: `query goto_definition use:foo` expects `def:foo`.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // target is the marker to query from (e.g., "use:foo")
    let Some(source_marker) = session.markers.range(&exp.target) else {
        return TestResult::Failed {
            message: format!("source marker '{}' not found", exp.target),
        };
    };

    // content is the expected result marker (e.g., "def:foo")
    let expected_marker = exp.content.trim();
    let Some(expected_def) = session.markers.range(expected_marker) else {
        return TestResult::Failed {
            message: format!("expected marker '{expected_marker}' not found"),
        };
    };

    let offset = source_marker.span.start;
    let result = query::goto_definition(&session.session, session.file_id, offset);

    match result {
        Some(def_result) => {
            if def_result.locations.is_empty() {
                return TestResult::Failed {
                    message: format!("goto_definition at offset {offset} returned empty result"),
                };
            }

            let found_match = def_result.locations.iter().any(|loc| {
                loc.start == expected_def.span.start && loc.end == expected_def.span.end
            });

            if !found_match {
                TestResult::Failed {
                    message: format!(
                        "goto_definition at '{}' returned wrong location: expected {:?}, got {:?}",
                        exp.target, expected_def.span, def_result.locations[0]
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("goto_definition at '{}' returned None", exp.target),
        },
    }
}
