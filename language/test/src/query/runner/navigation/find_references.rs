use destack_source::Span;
use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{file_for, format_span_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a find_references test.
///
/// Tests that querying from a marker finds the expected number of references.
/// Format: `query find_references def:foo` with content showing expected count or markers.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: check reference_count expectations from markers
    for (marker_name, expected_count) in &session.markers.expectations.reference_count {
        let Some(marker) = session.markers.range(marker_name) else {
            return TestResult::Failed {
                message: format!("marker '{marker_name}' not found"),
            };
        };

        // resolve marker file and run query
        let file_id = marker.span.file;
        let offset = marker.span.start;
        let declaration_span = query::goto_definition(&session.session, file_id, offset)
            .and_then(|result| result.locations.first().copied());
        let result = query::find_references(&session.session, file_id, offset, true);

        match result {
            Some(refs) => {
                // validate invariants before count comparisons
                if let Err(message) =
                    validate_reference_invariants(session, &refs.references, declaration_span)
                {
                    return TestResult::Failed { message };
                }

                if refs.len() != *expected_count {
                    return TestResult::Failed {
                        message: format!(
                            "find_references at '{marker_name}' returned {} references, expected {expected_count}",
                            refs.len(),
                        ),
                    };
                }
            }
            None => {
                return TestResult::Failed {
                    message: format!("find_references at '{marker_name}' returned None",),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let Some(source_marker) = session.markers.range(&exp.target) else {
        return TestResult::Failed {
            message: format!("source marker '{}' not found", exp.target),
        };
    };

    // resolve source file and offset
    let file_id = source_marker.span.file;
    let offset = source_marker.span.start;
    let declaration_span = query::goto_definition(&session.session, file_id, offset)
        .and_then(|result| result.locations.first().copied());

    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let result = query::find_references(&session.session, file_id, offset, true);
        return TestResult::Failed {
            message: format!(
                "find_references expectation is empty at '{}', got: {:?}",
                exp.target,
                result.map(|r| r.len())
            ),
        };
    }

    // none marker means we expect no results
    if content == "<none>" {
        let result = query::find_references(&session.session, file_id, offset, true);
        return match result {
            None => TestResult::Passed,
            Some(refs) => {
                // validate invariants when we do get a result
                if let Err(message) =
                    validate_reference_invariants(session, &refs.references, declaration_span)
                {
                    return TestResult::Failed { message };
                }

                if refs.is_empty() {
                    TestResult::Passed
                } else {
                    TestResult::Failed {
                        message: format!(
                            "find_references at '{}' expected no results, got {}",
                            exp.target,
                            refs.len()
                        ),
                    }
                }
            }
        };
    }

    // parse expected count from content (e.g., "3" or "count: 3")
    let count_str = content.strip_prefix("count:").unwrap_or(content).trim();
    if let Ok(expected_count) = count_str.parse::<usize>() {
        let result = query::find_references(&session.session, file_id, offset, true);
        return match result {
            Some(refs) => {
                // validate invariants before count comparisons
                if let Err(message) =
                    validate_reference_invariants(session, &refs.references, declaration_span)
                {
                    return TestResult::Failed { message };
                }

                if refs.len() != expected_count {
                    TestResult::Failed {
                        message: format!(
                            "find_references at '{}' returned {} references, expected {expected_count}",
                            exp.target,
                            refs.len(),
                        ),
                    }
                } else {
                    TestResult::Passed
                }
            }
            None => TestResult::Failed {
                message: format!("find_references at '{}' returned None", exp.target),
            },
        };
    }

    let result = query::find_references(&session.session, file_id, offset, true);
    let Some(refs) = result else {
        return TestResult::Failed {
            message: format!("find_references at '{}' returned None", exp.target),
        };
    };

    // validate reference invariants before comparing expectations
    if let Err(message) = validate_reference_invariants(session, &refs.references, declaration_span)
    {
        return TestResult::Failed { message };
    }

    // prefer protocol shaped snapshots when the expectation is structured
    if is_snapshot_expectation(content) {
        let actual_lines = format_reference_snapshot(session, &refs.references, declaration_span);
        let actual_snapshot = normalize_expected_snapshot(&actual_lines.join("\n"));
        let expected_snapshot = normalize_expected_snapshot(content);

        if actual_snapshot != expected_snapshot {
            return TestResult::Failed {
                message: format!(
                    "find_references snapshot mismatch at '{}'\n\nexpected:\n{}\n\nactual:\n{}",
                    exp.target, expected_snapshot, actual_snapshot
                ),
            };
        }

        return TestResult::Passed;
    }

    // parse expected markers from content
    let expected_markers: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    let mut expected_spans = Vec::new();
    for marker in &expected_markers {
        let Some(range) = session.markers.range(marker) else {
            return TestResult::Failed {
                message: format!("expected marker '{marker}' not found"),
            };
        };
        expected_spans.push(range.span);
    }

    // compare marker expectations against actual references
    let actual_spans = normalized_spans(&refs.references, declaration_span);
    let expected_spans = normalized_spans(&expected_spans, declaration_span);

    if actual_spans.len() != expected_spans.len() {
        return TestResult::Failed {
            message: format!(
                "find_references at '{}' returned {} references, expected {}",
                exp.target,
                actual_spans.len(),
                expected_spans.len()
            ),
        };
    }

    for expected in &expected_spans {
        let found = actual_spans.iter().any(|reference| reference == expected);
        if !found {
            let actual_lines = format_reference_snapshot(session, &actual_spans, declaration_span);
            return TestResult::Failed {
                message: format!(
                    "find_references at '{}' missing expected span {:?}\nactual:\n{}",
                    exp.target,
                    expected,
                    actual_lines.join("\n")
                ),
            };
        }
    }

    TestResult::Passed
}

/// Validate basic reference invariants.
fn validate_reference_invariants(
    session: &QueryTestSession,
    references: &[Span],
    declaration_span: Option<Span>,
) -> Result<(), String> {
    let mut errors = Vec::new();

    // check that spans are within the source and nonempty
    for span in references {
        let Some(file) = file_for(session, span.file) else {
            errors.push(format!(
                "reference file {:?} not found in session",
                span.file
            ));
            continue;
        };
        let source_len = u32::try_from(file.source.len()).unwrap_or(u32::MAX);

        // ensure the span start does not exceed the end
        if span.start > span.end {
            errors.push(format!(
                "{}: reference start {} is after end {}",
                file.name, span.start, span.end
            ));
        }

        // ensure the span end stays within the source bounds
        if span.end > source_len {
            errors.push(format!(
                "{}: reference end {} exceeds source length {}",
                file.name, span.end, source_len
            ));
        }
    }

    // normalize spans before duplicate checks
    let normalized = normalized_spans(references, declaration_span);

    // check for duplicate references
    for window in normalized.windows(2) {
        // compare adjacent spans for duplicates
        if window[0] == window[1] {
            errors.push(format!("duplicate reference span {:?}", window[0]));
        }
    }

    // check that the declaration is first when it is present in results
    if let Some(decl) = declaration_span {
        let declaration_present = normalized
            .iter()
            .any(|span| span.file == decl.file && span.start == decl.start && span.end == decl.end);
        if let (true, Some(first)) = (declaration_present, normalized.first()) {
            // ensure the declaration remains the first entry
            if first.file != decl.file || first.start != decl.start || first.end != decl.end {
                errors.push(format!(
                    "declaration is not first: expected {decl:?}, got {first:?}"
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "find_references invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Normalize spans into a deterministic order.
fn normalized_spans(spans: &[Span], declaration_span: Option<Span>) -> Vec<Span> {
    let mut spans = spans.to_vec();

    // keep the declaration first while sorting all other references
    let declaration_span = declaration_span.filter(|decl| {
        spans
            .iter()
            .any(|span| span.file == decl.file && span.start == decl.start && span.end == decl.end)
    });
    if let Some(decl) = declaration_span {
        spans.retain(|span| {
            !(span.file == decl.file && span.start == decl.start && span.end == decl.end)
        });
    }

    spans.sort_by_key(|span| (span.file.0, span.start, span.end));
    spans.dedup_by(|left, right| {
        left.file == right.file && left.start == right.start && left.end == right.end
    });

    if let Some(decl) = declaration_span {
        spans.insert(0, decl);
    }

    spans
}

/// Decide whether an expectation is a structured snapshot.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains("range=") || line.contains("file=") || line.contains(".ds:"))
}

/// Format references into a protocol shaped snapshot.
fn format_reference_snapshot(
    session: &QueryTestSession,
    references: &[Span],
    declaration_span: Option<Span>,
) -> Vec<String> {
    // normalize spans so snapshot output is deterministic
    let references = normalized_spans(references, declaration_span);

    // allocate snapshot lines for each reference
    let mut lines = Vec::new();

    // render each reference span into a protocol line
    for span in references {
        lines.push(format_reference_line(session, span));
    }
    lines
}

/// Format a single reference line.
fn format_reference_line(session: &QueryTestSession, span: Span) -> String {
    // resolve a stable file name for the span
    let file_name = file_for(session, span.file)
        .map(|file| file.name.as_str())
        .unwrap_or("<unknown>");

    // format the span range in line and column form
    let range = format_span(session, span);
    format!("{file_name}:{range}")
}

/// Format a span as a 1 based line and column range.
fn format_span(session: &QueryTestSession, span: Span) -> String {
    // resolve the source text for this span's file
    let source = source_for_file(session, span.file);

    // format the span using the shared line and column helper
    format_span_line_col(source, span)
}
