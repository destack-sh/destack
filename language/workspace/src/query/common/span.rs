use std::collections::HashSet;
use std::panic;
use std::sync::Arc;

use destack_dir::{self as dir, LocalNodeIdAny};
use destack_source::{EnclosingSpan, FileContent, FileId, Span};
use parking_lot::RwLock;

use super::QueryContext;
use super::identifier::is_identifier_byte;
use crate::program::{ModuleAst, ModuleDir};
use crate::{Module, Session};

/// Get a module by FileId.
pub fn get_module_by_file_id(session: &Session, file_id: FileId) -> Option<Arc<RwLock<Module>>> {
    session.modules.get_by_file_id(file_id)
}

/// Get the span of a DIR node by mapping through AST source map.
pub fn get_dir_node_span(
    ast: &ModuleAst,
    dir: &ModuleDir,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    let dir_tree = dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);
    drop(dir_tree);

    // get the span from AST source map
    Some(ast.tree.source_map.get(ast_node_id))
}

/// Get the main span of a DIR node (e.g., identifier for declarations).
/// Falls back to full span if no main span is set.
pub fn get_dir_node_main_span(
    ast: &ModuleAst,
    dir: &ModuleDir,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    let dir_tree = dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);
    drop(dir_tree);

    // try to get the main span first (e.g., identifier span for declarations)
    Some(ast.tree.source_map.get_main_or_enclosing(ast_node_id))
}

/// Find an identifier span within a larger span using identifier boundaries.
pub(crate) fn find_identifier_span_in_span(
    session: &Session,
    full_span: Span,
    name: &str,
) -> Option<Span> {
    let needle = name.as_bytes();

    // return the full span when the name is empty
    if needle.is_empty() {
        return Some(full_span);
    }

    // read source content for the span
    let file = session.files.get(full_span.file);
    let content = match &file.content {
        FileContent::Text { content } => content.as_str(),
        FileContent::Json { content, .. } => content.as_str(),
        _ => return Some(full_span),
    };

    // slice the span from the source text
    let start = full_span.start as usize;
    let end = full_span.end as usize;
    let Some(slice) = content.get(start..end) else {
        return Some(full_span);
    };

    // search for the identifier with basic identifier boundaries
    let hay = slice.as_bytes();
    let mut index = 0usize;
    while index + needle.len() <= hay.len() {
        let rel = find_bytes(&hay[index..], needle)?;
        let candidate_start = index + rel;
        let candidate_end = candidate_start + needle.len();

        if has_identifier_boundaries(hay, candidate_start, candidate_end) {
            let match_start = full_span.start + candidate_start as u32;
            let match_end = full_span.start + candidate_end as u32;
            return Some(Span::new(full_span.file, match_start, match_end));
        }

        index = candidate_end;
    }

    Some(full_span)
}

/// Resolve the span for a DIR node within a query context.
pub(crate) fn span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Span {
    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ctx.ast.tree.source_map.get(source_id);

    Span::new(ctx.file_id, ast_span.start, ast_span.end)
}

/// Resolve the main span for a DIR node when available
pub(crate) fn main_span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Option<Span> {
    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ctx.ast.tree.source_map.get_main(source_id)?;

    Some(Span::new(ctx.file_id, ast_span.start, ast_span.end))
}

/// Resolve the main or enclosing span for a DIR node
pub(crate) fn main_or_enclosing_span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Span {
    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ctx.ast.tree.source_map.get_main_or_enclosing(source_id);

    Span::new(ctx.file_id, ast_span.start, ast_span.end)
}

/// Resolve the span for a DIR node and guard against panics
pub(crate) fn span_for_dir_node_safe(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: u32,
) -> Option<Span> {
    // resolve the ast node id and guard source map access
    let ast_node_id = dir_tree.get_source(node_id);
    let full_span = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        ctx.ast.tree.source_map.get(ast_node_id)
    }));
    let Ok(full_span) = full_span else {
        return None;
    };

    Some(Span::new(ctx.file_id, full_span.start, full_span.end))
}

/// Check whether a span fully contains another span
pub(crate) fn span_contains_span(parent: Span, child: Span) -> bool {
    parent.start <= child.start && parent.end >= child.end
}

/// Resolve the span of a binding name within a larger pattern span.
pub(crate) fn binding_name_span(source: &str, span: Span, name: &str) -> Span {
    // slice the source to the pattern span
    let slice = source
        .get(span.start as usize..span.end as usize)
        .unwrap_or("");

    // locate the binding name within the span
    if let Some((rel_start, rel_end)) = find_identifier_span_in_slice(slice, name) {
        let name_start = span.start.saturating_add(rel_start as u32);
        let name_end = span.start.saturating_add(rel_end as u32);
        return Span::new(span.file, name_start, name_end);
    }

    // fall back to trimming trailing whitespace
    let trimmed_end = trim_span_end(source, span.start, span.end);
    Span::new(span.file, span.start, trimmed_end)
}

/// Trim trailing whitespace from a span end offset.
pub(crate) fn trim_span_end(source: &str, start: u32, end: u32) -> u32 {
    // walk backwards from the end while whitespace is present
    let mut idx = end as usize;
    let min = start as usize;
    let bytes = source.as_bytes();
    while idx > min {
        let Some(byte) = bytes.get(idx.saturating_sub(1)) else {
            break;
        };
        if byte.is_ascii_whitespace() {
            idx = idx.saturating_sub(1);
        } else {
            break;
        }
    }

    idx as u32
}

/// Extract the string literal prefix before a cursor offset.
pub(crate) fn extract_string_literal_prefix(source: &str, span: Span, offset: u32) -> String {
    // clamp the span within the source
    let start = span.start as usize;
    let end = span.end.min(source.len() as u32) as usize;
    if start >= end {
        return String::new();
    }

    // extract the literal text and cursor position
    let literal = &source[start..end];
    let offset_in_literal = offset.saturating_sub(span.start) as usize;

    // compute the content boundaries inside quotes
    let (content_start, content_end) = match literal.as_bytes().first().copied() {
        Some(b'"') | Some(b'\'') => {
            let end = literal
                .as_bytes()
                .last()
                .copied()
                .filter(|b| *b == b'"' || *b == b'\'')
                .map(|_| literal.len().saturating_sub(1))
                .unwrap_or(literal.len());
            (1, end)
        }
        _ => (0, literal.len()),
    };

    if offset_in_literal <= content_start {
        return String::new();
    }

    // return the prefix up to the cursor
    let prefix_end = offset_in_literal.min(content_end);
    literal[content_start..prefix_end].to_string()
}

/// Collect enclosing spans and sort from innermost to outermost.
pub(crate) fn sorted_enclosing_spans(
    ctx: &QueryContext<'_>,
    start: u32,
    end: u32,
) -> Vec<EnclosingSpan> {
    // collect enclosing spans from the source map
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(start, end);

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Collect enclosing spans at the cursor and previous byte.
pub(crate) fn enclosing_spans_with_previous(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    // collect enclosing spans at the cursor position
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);

    // include enclosing spans at the previous byte for boundary cases
    if offset > 0 {
        let previous_offset = offset - 1;
        let mut previous = ctx
            .ast
            .tree
            .source_map
            .get_enclosing_spans(previous_offset, previous_offset);
        enclosing.append(&mut previous);
    }

    // deduplicate by span index when we have overlapping collections
    if !enclosing.is_empty() {
        let mut seen = HashSet::new();
        enclosing.retain(|span| seen.insert(span.idx));
    }

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Sort spans by file and position and remove duplicates.
pub(crate) fn sort_and_dedup_spans(spans: &mut Vec<Span>) {
    // sort spans for stable ordering and deduplication
    spans.sort_by(|left, right| {
        (left.file.0, left.start, left.end).cmp(&(right.file.0, right.start, right.end))
    });

    // remove exact duplicate spans
    spans.dedup_by(|left, right| {
        left.file == right.file && left.start == right.start && left.end == right.end
    });
}

/// Find a byte slice within another byte slice.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Find a name in a slice while enforcing identifier boundaries.
fn find_identifier_span_in_slice(slice: &str, name: &str) -> Option<(usize, usize)> {
    let needle = name.as_bytes();

    if needle.is_empty() {
        return None;
    }

    let hay = slice.as_bytes();
    let mut index = 0usize;
    while index + needle.len() <= hay.len() {
        let rel = find_bytes(&hay[index..], needle)?;
        let candidate_start = index + rel;
        let candidate_end = candidate_start + needle.len();

        if has_identifier_boundaries(hay, candidate_start, candidate_end) {
            return Some((candidate_start, candidate_end));
        }

        index = candidate_end;
    }

    None
}

/// Check identifier boundaries around a candidate match.
fn has_identifier_boundaries(source: &[u8], start: usize, end: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|idx| source.get(idx))
        .copied();
    let after = source.get(end).copied();

    let before_ok = before.is_none_or(|b| !is_identifier_byte(b));
    let after_ok = after.is_none_or(|b| !is_identifier_byte(b));

    before_ok && after_ok
}
