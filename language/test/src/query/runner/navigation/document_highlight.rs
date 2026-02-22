use destack_source::{FileId, Span};
use destack_workspace::query;
use destack_workspace::query::{DocumentHighlight, HighlightKind};

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_highlight test.
///
/// Tests that querying from a marker highlights the expected ranges.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fall back to marker based expectations when no query block exists
    for (cursor_idx, expected_highlights) in &session.markers.expectations.highlights {
        // resolve the cursor position for this expectation
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };

        // run the highlight query at the cursor position
        let highlights =
            query::document_highlights(&session.session, session.file_id, cursor.offset);

        // compare the expected highlight count first for clearer errors
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

        // require each expected highlight span to appear in the results
        for exp in expected_highlights {
            let found = highlights
                .iter()
                .any(|h| h.range.start == exp.start && h.range.end == exp.end);

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
    // resolve the target file and offset from a cursor or marker
    let (file_id, offset) = if exp.target.starts_with('$') {
        let cursor_idx: usize = exp.target[1..].parse().unwrap_or(0);
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found"),
            };
        };
        (session.file_id, cursor.offset)
    } else {
        let Some(marker) = session.markers.range(&exp.target) else {
            return TestResult::Failed {
                message: format!("marker '{}' not found", exp.target),
            };
        };
        (marker.span.file, marker.span.start)
    };

    // require nonempty expectations so failures are explicit
    let content = exp.content.trim();
    if content.is_empty() {
        let highlights = query::document_highlights(&session.session, file_id, offset);
        return TestResult::Failed {
            message: format!(
                "document_highlight expectation is empty at '{}', got {} highlights",
                exp.target,
                highlights.len()
            ),
        };
    }

    // run the highlight query once for all expectation modes
    let highlights = query::document_highlights(&session.session, file_id, offset);

    // validate invariants before any comparisons
    if let Err(message) = validate_highlight_invariants(session, file_id, &highlights) {
        return TestResult::Failed { message };
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        return if highlights.is_empty() {
            TestResult::Passed
        } else {
            TestResult::Failed {
                message: format!(
                    "document_highlight at '{}' expected no highlights, got {}",
                    exp.target,
                    highlights.len()
                ),
            }
        };
    }

    // prefer structured snapshots when the expectation looks like a snapshot
    if is_snapshot_expectation(content) {
        return run_snapshot_expectation(session, &highlights, content);
    }

    // allow legacy numeric count expectations
    if let Ok(expected_count) = content.parse::<usize>() {
        return if highlights.len() != expected_count {
            TestResult::Failed {
                message: format!(
                    "document_highlight at '{}' returned {} highlights, expected {expected_count}",
                    exp.target,
                    highlights.len(),
                ),
            }
        } else {
            TestResult::Passed
        };
    }

    // parse expected markers from the content for marker based expectations
    let expected_markers: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    // resolve marker spans into expected spans
    let mut expected_spans = Vec::new();
    for marker in &expected_markers {
        let Some(range) = session.markers.range(marker) else {
            return TestResult::Failed {
                message: format!("expected marker '{marker}' not found"),
            };
        };
        expected_spans.push(range.span);
    }

    // check count agreement before comparing individual spans
    if highlights.len() != expected_spans.len() {
        return TestResult::Failed {
            message: format!(
                "document_highlight at '{}' returned {} highlights, expected {}",
                exp.target,
                highlights.len(),
                expected_spans.len()
            ),
        };
    }

    // require each expected span to appear in the highlight results
    for expected in &expected_spans {
        let found = highlights.iter().any(|highlight| {
            highlight.range.start == expected.start && highlight.range.end == expected.end
        });

        if !found {
            return TestResult::Failed {
                message: format!(
                    "document_highlight at '{}' missing expected span {:?}",
                    exp.target, expected
                ),
            };
        }
    }

    TestResult::Passed
}

/// Decide whether a document highlight expectation is snapshot shaped.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains(".ds:") || line.contains("kind="))
}

/// Run a structured snapshot expectation for document highlights.
fn run_snapshot_expectation(
    session: &QueryTestSession,
    highlights: &[DocumentHighlight],
    expected: &str,
) -> TestResult {
    // format the highlight results into deterministic snapshot lines
    let actual_snapshot =
        normalize_expected_snapshot(&format_highlight_snapshot(session, highlights).join("\n"));
    let expected_snapshot = normalize_expected_snapshot(expected);

    if actual_snapshot == expected_snapshot {
        return TestResult::Passed;
    }

    TestResult::Failed {
        message: format!(
            "document_highlight snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
        ),
    }
}

/// Validate common invariants for document highlights.
fn validate_highlight_invariants(
    session: &QueryTestSession,
    file_id: FileId,
    highlights: &[DocumentHighlight],
) -> Result<(), String> {
    // resolve the source so we can check range bounds
    let source = source_for_file(session, file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // collect any invariant violations across all highlights
    let mut errors = Vec::new();
    for highlight in highlights {
        validate_highlight_range(highlight.range, file_id, source_len, &mut errors);
    }

    if errors.is_empty() {
        return Ok(());
    }

    let message = errors.join("\n");
    Err(format!(
        "document_highlight invariants violated:\n{message}"
    ))
}

/// Validate a document highlight range.
fn validate_highlight_range(
    range: Span,
    file_id: FileId,
    source_len: u32,
    errors: &mut Vec<String>,
) {
    // require highlight ranges to point at the target file
    if range.file != file_id {
        errors.push(format!(
            "highlight file mismatch: expected {file_id:?}, got {:?}",
            range.file
        ));
    }

    // require ordered ranges within file bounds
    if range.start > range.end {
        errors.push(format!("highlight start > end: {range:?}"));
    }
    if range.start == range.end {
        errors.push(format!("highlight range is empty: {range:?}"));
    }
    if range.end > source_len {
        errors.push(format!(
            "highlight end out of bounds: end={}, len={source_len}",
            range.end
        ));
    }
}

/// Format highlights into a protocol shaped snapshot.
fn format_highlight_snapshot(
    session: &QueryTestSession,
    highlights: &[DocumentHighlight],
) -> Vec<String> {
    // normalize highlight ordering and drop duplicates
    let highlights = normalized_highlights(highlights);

    // render each highlight into a snapshot line
    let mut lines = Vec::new();
    for highlight in highlights {
        lines.push(format_highlight_line(session, &highlight));
    }
    lines
}

/// Normalize highlights into a stable sorted order.
fn normalized_highlights(highlights: &[DocumentHighlight]) -> Vec<DocumentHighlight> {
    // copy highlights so we can sort and deduplicate them
    let mut highlights = highlights.to_vec();

    // sort by range and kind for deterministic snapshot output
    highlights.sort_by_cached_key(highlight_snapshot_key);

    // drop exact duplicates so snapshots stay small
    highlights.dedup_by(|left, right| left.range == right.range && left.kind == right.kind);

    highlights
}

/// Build a stable sort key for highlight snapshots.
fn highlight_snapshot_key(highlight: &DocumentHighlight) -> (u32, u32, u8) {
    (
        highlight.range.start,
        highlight.range.end,
        highlight_kind_rank(highlight.kind),
    )
}

/// Format a single highlight snapshot line.
fn format_highlight_line(session: &QueryTestSession, highlight: &DocumentHighlight) -> String {
    // format the range using shared span formatting helpers
    let range = format_span_for_session(session, highlight.range);

    // format the kind as a stable lowercase tag
    let kind = highlight_kind_name(highlight.kind);
    format!("{range} kind={kind}")
}

/// Format a highlight kind as a stable lowercase name.
fn highlight_kind_name(kind: HighlightKind) -> &'static str {
    match kind {
        HighlightKind::Text => "text",
        HighlightKind::Read => "read",
        HighlightKind::Write => "write",
    }
}

/// Rank highlight kinds for deterministic sorting.
fn highlight_kind_rank(kind: HighlightKind) -> u8 {
    match kind {
        HighlightKind::Text => 0,
        HighlightKind::Read => 1,
        HighlightKind::Write => 2,
    }
}
