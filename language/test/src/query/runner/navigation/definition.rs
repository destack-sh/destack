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

/// Run a goto_type_definition test.
///
/// For each marker, verify it resolves to the expected type definition marker.
pub fn run_type_definition(
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no expectation for goto_type_definition".to_string(),
        };
    };

    run_type_definition_with_expectation(session, exp)
}

/// Run type definition with markdown expectation.
fn run_type_definition_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
) -> TestResult {
    // target is the marker to query from (e.g., "use:foo" or "$0")
    let offset = if exp.target.starts_with('$') {
        let cursor_idx: usize = exp
            .target
            .strip_prefix('$')
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };
        cursor.offset
    } else {
        let Some(source_marker) = session.markers.range(&exp.target) else {
            return TestResult::Failed {
                message: format!("source marker '{}' not found", exp.target),
            };
        };
        source_marker.span.start
    };

    // content is the expected result marker (e.g., "def:MyClass")
    let expected_marker = exp.content.trim();

    // "<none>" means we expect no result
    if expected_marker == "<none>" {
        let result = query::goto_type_definition(&session.session, session.file_id, offset);
        return match result {
            None => TestResult::Passed,
            Some(def_result) => TestResult::Failed {
                message: format!(
                    "goto_type_definition expected None, got {:?}",
                    def_result.locations
                ),
            },
        };
    }

    let Some(expected_def) = session.markers.range(expected_marker) else {
        return TestResult::Failed {
            message: format!("expected marker '{expected_marker}' not found"),
        };
    };

    let result = query::goto_type_definition(&session.session, session.file_id, offset);

    match result {
        Some(def_result) => {
            if def_result.locations.is_empty() {
                return TestResult::Failed {
                    message: format!(
                        "goto_type_definition at offset {offset} returned empty result"
                    ),
                };
            }

            let found_match = def_result.locations.iter().any(|loc| {
                loc.start == expected_def.span.start && loc.end == expected_def.span.end
            });

            if !found_match {
                TestResult::Failed {
                    message: format!(
                        "goto_type_definition at '{}' returned wrong location: expected {:?}, got {:?}",
                        exp.target, expected_def.span, def_result.locations[0]
                    ),
                }
            } else {
                TestResult::Passed
            }
        }
        None => TestResult::Failed {
            message: format!("goto_type_definition at '{}' returned None", exp.target),
        },
    }
}
