use destack_source::Span;
use destack_workspace::query;
use destack_workspace::query::{DocumentLink, DocumentLinkTarget};

use crate::harness::TestResult;
use crate::query::runner::snapshot::normalize_expected_snapshot;
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_link test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no document_links expectation provided".to_string(),
        };
    };

    // run the query for the current file
    let links = query::document_links(&session.session, session.file_id);

    // require nonempty expectations so failures are explicit
    let expected_content = exp.content.trim();
    if expected_content.is_empty() {
        let actual_lines = format_document_link_snapshot(session, &links).join("\n");
        return TestResult::Failed {
            message: format!(
                "document_link expectation is empty, but query returned:\n{actual_lines}"
            ),
        };
    }

    // validate invariants before any comparisons
    if let Err(message) = validate_link_invariants(session, &links) {
        return TestResult::Failed { message };
    }

    // use structured snapshots when the expectation looks like a snapshot
    if is_snapshot_expectation(expected_content) {
        return run_snapshot_expectation(session, &links, expected_content);
    }

    // fall back to legacy count expectations when given a number
    let expected_count: usize = expected_content.parse().unwrap_or(0);
    if links.len() == expected_count {
        return TestResult::Passed;
    }

    TestResult::Failed {
        message: format!(
            "expected {expected_count} document links, found {}",
            links.len()
        ),
    }
}

/// Decide whether a document link expectation is snapshot shaped.
fn is_snapshot_expectation(expected: &str) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| line.contains(".ds:") || line.contains("target="))
}

/// Run a structured snapshot expectation for document links.
fn run_snapshot_expectation(
    session: &QueryTestSession,
    links: &[DocumentLink],
    expected: &str,
) -> TestResult {
    // format the actual snapshot into deterministic lines
    let actual_snapshot =
        normalize_expected_snapshot(&format_document_link_snapshot(session, links).join("\n"));
    let expected_snapshot = normalize_expected_snapshot(expected);

    if actual_snapshot == expected_snapshot {
        return TestResult::Passed;
    }

    TestResult::Failed {
        message: format!(
            "document_link snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
        ),
    }
}

/// Validate common invariants for document links.
fn validate_link_invariants(
    session: &QueryTestSession,
    links: &[DocumentLink],
) -> Result<(), String> {
    // resolve the source so we can check range bounds
    let source = source_for_file(session, session.file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // collect violations across all links
    let mut errors = Vec::new();
    for link in links {
        validate_link_range(link.range, session.file_id, source_len, &mut errors);
        validate_link_target(&link.target, &mut errors);
    }

    if errors.is_empty() {
        return Ok(());
    }

    let message = errors.join("\n");
    Err(format!("document_link invariants violated:\n{message}"))
}

/// Validate a document link range.
fn validate_link_range(
    range: Span,
    file_id: destack_source::FileId,
    source_len: u32,
    errors: &mut Vec<String>,
) {
    // require ranges to point at the requested file
    if range.file != file_id {
        errors.push(format!(
            "range file mismatch: expected {file_id:?}, got {:?}",
            range.file
        ));
    }

    // require ordered ranges within file bounds
    if range.start > range.end {
        errors.push(format!("range start > end: {:?}", range));
    }
    if range.end > source_len {
        errors.push(format!(
            "range end out of bounds: end={}, len={source_len}",
            range.end
        ));
    }
}

/// Validate a document link target.
fn validate_link_target(target: &DocumentLinkTarget, errors: &mut Vec<String>) {
    match target {
        DocumentLinkTarget::File { path } => {
            if path.trim().is_empty() {
                errors.push("file target path is empty".to_string());
            }
        }
        DocumentLinkTarget::Url { url } => {
            if url.trim().is_empty() {
                errors.push("url target is empty".to_string());
            }
        }
        DocumentLinkTarget::Position { path, .. } => {
            if path.trim().is_empty() {
                errors.push("position target path is empty".to_string());
            }
        }
    }
}

/// Format document links into a protocol shaped snapshot.
fn format_document_link_snapshot(
    session: &QueryTestSession,
    links: &[DocumentLink],
) -> Vec<String> {
    // sort links into a stable order for snapshots
    let mut sorted = links.to_vec();
    sorted.sort_by_cached_key(link_snapshot_key);

    // render each link into a snapshot line
    let mut lines = Vec::new();
    for link in sorted {
        lines.push(format_document_link_line(session, &link));
    }
    lines
}

/// Build a stable sort key for snapshot ordering.
fn link_snapshot_key(link: &DocumentLink) -> (u32, u32, u8, String) {
    let (target_rank, target_key) = link_target_key(&link.target);
    (link.range.start, link.range.end, target_rank, target_key)
}

/// Format a single document link snapshot line.
fn format_document_link_line(session: &QueryTestSession, link: &DocumentLink) -> String {
    // format the link range using shared span formatting
    let range = format_span_for_session(session, link.range);

    // format the target into a deterministic representation
    let target = format_link_target(&link.target);
    match &link.tooltip {
        Some(tooltip) if !tooltip.is_empty() => {
            format!("{range} target={target} tooltip={tooltip}")
        }
        _ => format!("{range} target={target}"),
    }
}

/// Format a document link target for snapshot output.
fn format_link_target(target: &DocumentLinkTarget) -> String {
    match target {
        DocumentLinkTarget::File { path } => format!("file:{path}"),
        DocumentLinkTarget::Url { url } => format!("url:{url}"),
        DocumentLinkTarget::Position { path, line, column } => {
            let line = line.saturating_add(1);
            let column = column.saturating_add(1);
            format!("pos:{path}:{line}:{column}")
        }
    }
}

/// Provide a sortable key for a document link target.
fn link_target_key(target: &DocumentLinkTarget) -> (u8, String) {
    match target {
        DocumentLinkTarget::File { path } => (0, path.clone()),
        DocumentLinkTarget::Url { url } => (1, url.clone()),
        DocumentLinkTarget::Position { path, line, column } => {
            (2, format!("{path}:{line}:{column}"))
        }
    }
}
