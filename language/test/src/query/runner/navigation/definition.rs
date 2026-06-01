use destack_query as query;
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{
    compare_snapshot, looks_like_span_snapshot, normalize_expected_snapshot,
};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_definition test.
///
/// For each use marker, verify it resolves to the expected def marker.
/// With markdown format, the expectation specifies target marker and expected result.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no goto_definition expectation provided".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation: `query goto_definition use:foo` expects `def:foo`.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // normalize the expected content for comparison
    let expected_content = exp.content.trim();

    // empty expectation is an error
    if expected_content.is_empty() {
        return CaseResult::Failed {
            message: format!("goto_definition expectation at '{}' is empty", exp.target),
        };
    }

    // run goto definition at the resolved position
    let ctx = session.module_context(file_id);
    let targets = ctx.goto_definition(offset);
    let locations = navigation_target_spans(&targets);

    // "<none>" means we expect no result
    if expected_content == "<none>" {
        return if locations.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "goto_definition at '{}' expected no results, got {} locations",
                    exp.target,
                    locations.len()
                ),
            }
        };
    }

    // require at least one definition location
    if locations.is_empty() {
        return CaseResult::Failed {
            message: format!("goto_definition at '{}' returned empty result", exp.target),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_definition_invariants(session, &locations) {
        return CaseResult::Failed { message };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected_content, &["range="]) {
        let actual_snapshot = snapshot_locations(session, &locations);
        return compare_snapshot(
            &format!("goto_definition at '{}'", exp.target),
            &actual_snapshot,
            expected_content,
        );
    }

    // resolve the expected definition marker
    let Some(expected_def) = session.markers.range(expected_content) else {
        return CaseResult::Failed {
            message: format!("expected marker '{expected_content}' not found"),
        };
    };

    if let Err(message) = require_exact_marker_match(
        "goto_definition",
        exp.target.clone(),
        &locations,
        expected_def.span,
    ) {
        return CaseResult::Failed { message };
    }

    CaseResult::Passed
}

/// Run a goto_type_definition test.
///
/// For each marker, verify it resolves to the expected type definition marker.
pub fn run_type_definition(
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no expectation for goto_type_definition".to_string(),
        };
    };

    run_type_definition_with_expectation(session, exp)
}

/// Run a goto_declaration test.
pub fn run_declaration(
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no expectation for goto_declaration".to_string(),
        };
    };

    run_declaration_with_expectation(session, exp)
}

/// Run declaration with markdown expectation.
fn run_declaration_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // normalize the expected content for comparison
    let expected_content = exp.content.trim();

    // empty expectation is an error
    if expected_content.is_empty() {
        return CaseResult::Failed {
            message: format!("goto_declaration expectation at '{}' is empty", exp.target),
        };
    }

    // run goto declaration at the resolved position
    let ctx = session.module_context(file_id);
    let targets = ctx.goto_declaration(offset);
    let locations = navigation_target_spans(&targets);

    // "<none>" means we expect no result
    if expected_content == "<none>" {
        return if locations.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "goto_declaration at '{}' expected no results, got {} locations",
                    exp.target,
                    locations.len()
                ),
            }
        };
    }

    // require at least one declaration location
    if locations.is_empty() {
        return CaseResult::Failed {
            message: format!("goto_declaration at '{}' returned empty result", exp.target),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_definition_invariants(session, &locations) {
        return CaseResult::Failed { message };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected_content, &["range="]) {
        let actual_snapshot = snapshot_locations(session, &locations);
        return compare_snapshot(
            &format!("goto_declaration at '{}'", exp.target),
            &actual_snapshot,
            expected_content,
        );
    }

    // resolve the expected declaration marker
    let Some(expected_def) = session.markers.range(expected_content) else {
        return CaseResult::Failed {
            message: format!("expected marker '{expected_content}' not found"),
        };
    };

    if let Err(message) = require_exact_marker_match(
        "goto_declaration",
        exp.target.clone(),
        &locations,
        expected_def.span,
    ) {
        return CaseResult::Failed { message };
    }

    CaseResult::Passed
}

/// Run type definition with markdown expectation.
fn run_type_definition_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
) -> CaseResult {
    // resolve the query position from the expectation target
    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };

    // normalize the expected content for comparison
    let expected_content = exp.content.trim();

    // empty expectation is an error
    if expected_content.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "goto_type_definition expectation at '{}' is empty",
                exp.target
            ),
        };
    }

    // run goto type definition at the resolved position
    let ctx = session.module_context(file_id);
    let targets = ctx.goto_type_definition(offset);
    let locations = navigation_target_spans(&targets);

    // "<none>" means we expect no result
    if expected_content == "<none>" {
        return if locations.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "goto_type_definition at '{}' expected no results, got {} locations",
                    exp.target,
                    locations.len()
                ),
            }
        };
    }

    // require at least one type definition location
    if locations.is_empty() {
        return CaseResult::Failed {
            message: format!(
                "goto_type_definition at '{}' returned empty result",
                exp.target
            ),
        };
    }

    // validate invariants before comparisons
    if let Err(message) = validate_definition_invariants(session, &locations) {
        return CaseResult::Failed { message };
    }

    // compare against protocol shaped snapshots when structured
    if looks_like_span_snapshot(expected_content, &["range="]) {
        let actual_snapshot = snapshot_locations(session, &locations);
        return compare_snapshot(
            &format!("goto_type_definition at '{}'", exp.target),
            &actual_snapshot,
            expected_content,
        );
    }

    // resolve the expected type definition marker
    let Some(expected_def) = session.markers.range(expected_content) else {
        return CaseResult::Failed {
            message: format!("expected marker '{expected_content}' not found"),
        };
    };

    if let Err(message) = require_exact_marker_match(
        "goto_type_definition",
        exp.target.clone(),
        &locations,
        expected_def.span,
    ) {
        return CaseResult::Failed { message };
    }

    CaseResult::Passed
}

/// Return the source spans for navigation targets.
fn navigation_target_spans(targets: &[query::NavigationTarget]) -> Vec<Span> {
    targets.iter().map(|target| target.target.span).collect()
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
    let mut previous: Option<(u128, u32, u32)> = None;
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
fn location_key(span: Span) -> (u128, u32, u32) {
    (span.file.0, span.start, span.end)
}

/// Require one exact location match for marker based expectations.
fn require_exact_marker_match(
    operation: &str,
    target: impl AsRef<str>,
    locations: &[Span],
    expected: Span,
) -> Result<(), String> {
    let target = target.as_ref();

    if locations.len() != 1 {
        return Err(format!(
            "{operation} at '{target}' returned {} locations, expected exactly 1 matching {:?}",
            locations.len(),
            expected
        ));
    }

    let actual = locations[0];
    if actual == expected {
        return Ok(());
    }

    Err(format!(
        "{operation} at '{target}' returned wrong location: expected {expected:?}, got {actual:?}"
    ))
}

#[cfg(test)]
mod tests {
    use destack_source::{FileId, Span};

    use super::require_exact_marker_match;

    #[test]
    fn test_require_exact_marker_match_rejects_extra_locations() {
        let expected = Span::new(FileId(0), 1, 2);
        let locations = [expected, Span::new(FileId(0), 3, 4)];

        let error =
            require_exact_marker_match("goto_definition", "use:value", &locations, expected)
                .unwrap_err();

        assert!(error.contains("returned 2 locations"));
    }

    #[test]
    fn test_require_exact_marker_match_rejects_wrong_location() {
        let expected = Span::new(FileId(0), 1, 2);
        let locations = [Span::new(FileId(0), 3, 4)];

        let error =
            require_exact_marker_match("goto_definition", "use:value", &locations, expected)
                .unwrap_err();

        assert!(error.contains("returned wrong location"));
    }
}
