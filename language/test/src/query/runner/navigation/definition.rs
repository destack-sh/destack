use destack_source::Span;
use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
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

    // fall back to inline reference targets when no markdown expectation is present
    for reference in session.markers.references() {
        let Some(target_name) = &reference.target else {
            continue;
        };

        // resolve the expected target marker
        let Some(expected_def) = session.markers.range(target_name) else {
            return TestResult::Failed {
                message: format!("target marker '{target_name}' not found"),
            };
        };

        // run goto definition at the reference position
        let file_id = reference.span.file;
        let offset = reference.span.start;
        let result = query::goto_definition(&session.session, file_id, offset);

        let Some(def_result) = result else {
            return TestResult::Failed {
                message: format!("goto_definition at offset {offset} returned None"),
            };
        };

        // require at least one definition location
        if def_result.locations.is_empty() {
            return TestResult::Failed {
                message: format!("goto_definition at offset {offset} returned empty result"),
            };
        }

        // validate invariants before matching expected spans
        if let Err(message) = validate_definition_invariants(session, &def_result.locations) {
            return TestResult::Failed { message };
        }

        // check that the expected target span is present
        let found_match = def_result
            .locations
            .iter()
            .any(|loc| loc.start == expected_def.span.start && loc.end == expected_def.span.end);

        if !found_match {
            return TestResult::Failed {
                message: format!(
                    "goto_definition at offset {offset} returned wrong location: expected {:?}, got {:?}",
                    expected_def.span, def_result.locations[0]
                ),
            };
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation: `query goto_definition use:foo` expects `def:foo`.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // normalize the expected content for comparison
    let expected_content = exp.content.trim();

    // empty expectation is an error
    if expected_content.is_empty() {
        return TestResult::Failed {
            message: format!("goto_definition expectation at '{}' is empty", exp.target),
        };
    }

    // run goto definition at the resolved position
    let result = query::goto_definition(&session.session, file_id, offset);

    // "<none>" means we expect no result
    if expected_content == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(def_result) if def_result.locations.is_empty() => TestResult::Passed,
            Some(def_result) => TestResult::Failed {
                message: format!(
                    "goto_definition at '{}' expected no results, got {} locations",
                    exp.target,
                    def_result.locations.len()
                ),
            },
        };
    }

    let Some(def_result) = result else {
        return TestResult::Failed {
            message: format!("goto_definition at '{}' returned None", exp.target),
        };
    };

    // require at least one definition location
    if def_result.locations.is_empty() {
        return TestResult::Failed {
            message: format!("goto_definition at '{}' returned empty result", exp.target),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_definition_invariants(session, &def_result.locations) {
        return TestResult::Failed { message };
    }

    // compare against protocol shaped snapshots when structured
    if is_snapshot_expectation(expected_content) {
        let actual_snapshot = snapshot_locations(session, &def_result.locations);
        let expected_snapshot = normalize_expected_snapshot(expected_content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "goto_definition snapshot mismatch at '{}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}",
                    exp.target
                ),
            };
        }

        return TestResult::Passed;
    }

    // resolve the expected definition marker
    let Some(expected_def) = session.markers.range(expected_content) else {
        return TestResult::Failed {
            message: format!("expected marker '{expected_content}' not found"),
        };
    };

    // check that the expected target span is present
    let found_match = def_result
        .locations
        .iter()
        .any(|loc| loc.start == expected_def.span.start && loc.end == expected_def.span.end);

    if !found_match {
        return TestResult::Failed {
            message: format!(
                "goto_definition at '{}' returned wrong location: expected {:?}, got {:?}",
                exp.target, expected_def.span, def_result.locations[0]
            ),
        };
    }

    TestResult::Passed
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
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return TestResult::Failed { message },
    };

    // normalize the expected content for comparison
    let expected_content = exp.content.trim();

    // empty expectation is an error
    if expected_content.is_empty() {
        return TestResult::Failed {
            message: format!(
                "goto_type_definition expectation at '{}' is empty",
                exp.target
            ),
        };
    }

    // run goto type definition at the resolved position
    let result = query::goto_type_definition(&session.session, file_id, offset);

    // "<none>" means we expect no result
    if expected_content == "<none>" {
        return match result {
            None => TestResult::Passed,
            Some(def_result) if def_result.locations.is_empty() => TestResult::Passed,
            Some(def_result) => TestResult::Failed {
                message: format!(
                    "goto_type_definition at '{}' expected no results, got {} locations",
                    exp.target,
                    def_result.locations.len()
                ),
            },
        };
    }

    let Some(def_result) = result else {
        return TestResult::Failed {
            message: format!("goto_type_definition at '{}' returned None", exp.target),
        };
    };

    // require at least one type definition location
    if def_result.locations.is_empty() {
        return TestResult::Failed {
            message: format!(
                "goto_type_definition at '{}' returned empty result",
                exp.target
            ),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_definition_invariants(session, &def_result.locations) {
        return TestResult::Failed { message };
    }

    // compare against protocol shaped snapshots when structured
    if is_snapshot_expectation(expected_content) {
        let actual_snapshot = snapshot_locations(session, &def_result.locations);
        let expected_snapshot = normalize_expected_snapshot(expected_content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "goto_type_definition snapshot mismatch at '{}'\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}",
                    exp.target
                ),
            };
        }

        return TestResult::Passed;
    }

    // resolve the expected type definition marker
    let Some(expected_def) = session.markers.range(expected_content) else {
        return TestResult::Failed {
            message: format!("expected marker '{expected_content}' not found"),
        };
    };

    // check that the expected target span is present
    let found_match = def_result
        .locations
        .iter()
        .any(|loc| loc.start == expected_def.span.start && loc.end == expected_def.span.end);

    if !found_match {
        return TestResult::Failed {
            message: format!(
                "goto_type_definition at '{}' returned wrong location: expected {:?}, got {:?}",
                exp.target, expected_def.span, def_result.locations[0]
            ),
        };
    }

    TestResult::Passed
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    // detect structured snapshots by range markers or span like digits
    expected.lines().map(str::trim).any(|line| {
        let has_range_marker = line.contains("range=");
        let has_digit_span = line.contains(':') && line.chars().any(|c| c.is_ascii_digit());
        has_range_marker || has_digit_span
    })
}

/// Format definition locations into a protocol shaped snapshot.
fn snapshot_locations(session: &QueryTestSession, locations: &[Span]) -> String {
    // format each location using the shared span formatter
    let snapshot = locations
        .iter()
        .map(|span| format_span_for_session(session, *span))
        .collect::<Vec<_>>()
        .join("\n");

    normalize_expected_snapshot(&snapshot)
}

/// Validate definition location invariants.
fn validate_definition_invariants(
    session: &QueryTestSession,
    locations: &[Span],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // validate span bounds for each location
    for span in locations {
        let source = source_for_file(session, span.file);
        let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

        // ensure the span start does not exceed the end
        if span.start > span.end {
            errors.push(format!(
                "definition span start {} is after end {}",
                span.start, span.end
            ));
        }

        // ensure the span end stays within the source bounds
        if span.end > source_len {
            errors.push(format!(
                "definition span end {} exceeds source length {}",
                span.end, source_len
            ));
        }
    }

    // validate ordering and duplicates
    let mut previous: Option<(u32, u32, u32)> = None;
    for span in locations {
        // build a stable ordering key for the span
        let key = location_key(*span);

        // compare against the previous key for ordering and duplicates
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "definition locations are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("duplicate definition location {key:?}"));
            }
        }

        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "goto_definition invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Build a stable ordering key for a span.
fn location_key(span: Span) -> (u32, u32, u32) {
    (span.file.0, span.start, span.end)
}
