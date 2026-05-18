use std::path::Path;

use destack_query as query;
use destack_query::{DocumentLink, DocumentLinkTarget};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{compare_snapshot, looks_like_span_snapshot};
use crate::query::runner::span::{format_span_for_session, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_link test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no document_links expectation provided".to_string(),
        };
    };

    // run the query for the current file
    let ctx = session.primary_module_context();
    let links = query::document_links(&ctx);

    // require nonempty expectations so failures are explicit
    let expected_content = exp.content.trim();
    if expected_content.is_empty() {
        let actual_lines = format_document_link_snapshot(session, &links).join("\n");
        return CaseResult::Failed {
            message: format!(
                "document_link expectation is empty, but query returned:\n{actual_lines}"
            ),
        };
    }

    // validate invariants before any comparisons
    if let Err(message) = validate_link_invariants(session, &links) {
        return CaseResult::Failed { message };
    }

    // "<none>" means we expect no links
    if expected_content == "<none>" {
        return if links.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!("expected no document links, found {}", links.len()),
            }
        };
    }

    // use structured snapshots when the expectation looks like a snapshot
    if looks_like_span_snapshot(expected_content, &["target=", "tooltip="]) {
        return run_snapshot_expectation(session, &links, expected_content);
    }

    CaseResult::Failed {
        message: format!(
            "document_link expectation '{expected_content}' is not a valid snapshot or <none>"
        ),
    }
}

/// Run a resolve_document_link test.
pub fn run_resolve(
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no resolve_document_link expectation provided".to_string(),
        };
    };

    let content = exp.content.trim();
    if content.is_empty() {
        return CaseResult::Failed {
            message: "resolve_document_link expectation is empty".to_string(),
        };
    }

    let ctx = session.primary_module_context();
    let links = query::document_links(&ctx);
    if links.is_empty() {
        return if content == "<none>" {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: "resolve_document_link expected a link, but none were returned"
                    .to_string(),
            }
        };
    }

    let (file_id, offset) = match resolve_query_position(session, &exp.target) {
        Ok(position) => position,
        Err(message) => return CaseResult::Failed { message },
    };
    if file_id != session.file_id {
        return CaseResult::Failed {
            message: "resolve_document_link only supports the primary file".to_string(),
        };
    }

    let index = exp.args.first().and_then(|arg| arg.parse::<usize>().ok());
    let Some(link) = select_link(&links, Some(offset), index) else {
        return CaseResult::Failed {
            message: "resolve_document_link could not select a link".to_string(),
        };
    };

    let resolved = query::resolve_document_link(link);
    let actual_line = format_document_link_line(session, &resolved);

    if looks_like_span_snapshot(content, &["target=", "tooltip="]) {
        return compare_snapshot("resolve_document_link", &actual_line, content);
    }

    if content == "<same>" {
        let original_line = format_document_link_line(session, link);
        return if original_line == actual_line {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!(
                    "resolve_document_link expected unchanged link\n\nexpected:\n{original_line}\n\nactual:\n{actual_line}"
                ),
            }
        };
    }

    CaseResult::Failed {
        message: format!(
            "resolve_document_link expectation did not match\n\nactual:\n{actual_line}"
        ),
    }
}

/// Run a structured snapshot expectation for document links.
fn run_snapshot_expectation(
    session: &QueryTestSession,
    links: &[DocumentLink],
    expected: &str,
) -> CaseResult {
    compare_snapshot(
        "document_link",
        &format_document_link_snapshot(session, links).join("\n"),
        expected,
    )
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
        errors.push(format!("range start > end: {range:?}"));
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
    let target = format_link_target(session, &link.target);
    match &link.tooltip {
        Some(tooltip) if !tooltip.is_empty() => {
            format!("{range} target={target} tooltip={tooltip}")
        }
        _ => format!("{range} target={target}"),
    }
}

/// Select a document link by offset or index.
fn select_link(
    links: &[DocumentLink],
    offset: Option<u32>,
    index: Option<usize>,
) -> Option<&DocumentLink> {
    if let Some(index) = index {
        return links.get(index);
    }

    if let Some(offset) = offset
        && let Some(link) = links
            .iter()
            .find(|link| link.range.start <= offset && link.range.end >= offset)
    {
        return Some(link);
    }

    if links.len() == 1 {
        return links.first();
    }

    None
}

/// Format a document link target for snapshot output.
fn format_link_target(session: &QueryTestSession, target: &DocumentLinkTarget) -> String {
    match target {
        DocumentLinkTarget::File { path } => {
            let normalized = normalize_link_path(session, path);
            format!("file:{normalized}")
        }
        DocumentLinkTarget::Url { url } => format!("url:{url}"),
        DocumentLinkTarget::Position { path, line, column } => {
            let line = line.saturating_add(1);
            let column = column.saturating_add(1);
            let normalized = normalize_link_path(session, path);
            format!("pos:{normalized}:{line}:{column}")
        }
    }
}

/// Normalize a link path for stable snapshots.
fn normalize_link_path(session: &QueryTestSession, path: &str) -> String {
    let path = Path::new(path);

    if let Ok(stripped) = path.strip_prefix(&session.root) {
        let normalized = stripped.to_string_lossy().to_string();
        let trimmed = normalized.trim_start_matches('/');
        return trimmed.to_string();
    }

    path.to_string_lossy().to_string()
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
