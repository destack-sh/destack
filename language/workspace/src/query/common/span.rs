use std::sync::Arc;

use destack_dir::LocalNodeIdAny;
use destack_source::{FileContent, FileId, Span};
use parking_lot::RwLock;

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

/// Check whether a byte is a simple identifier character.
pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Extract the identifier token at a given offset.
pub(crate) fn token_at_offset(session: &Session, file_id: FileId, offset: u32) -> Option<String> {
    // read the source content
    let file = session.files.get(file_id);
    let content = match &file.content {
        FileContent::Text { content } => content.as_str(),
        FileContent::Json { content, .. } => content.as_str(),
        _ => return None,
    };

    // clamp the offset to the file length
    let bytes = content.as_bytes();
    let mut index = offset as usize;
    if index >= bytes.len() {
        index = bytes.len().saturating_sub(1);
    }

    // require the cursor to be on an identifier byte
    let current = *bytes.get(index)?;
    if !is_identifier_byte(current) {
        return None;
    }

    // scan left to the start of the token
    let mut start = index;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }

    // scan right to the end of the token
    let mut end = index + 1;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }

    let slice = content.get(start..end)?;
    Some(slice.to_string())
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
