use std::collections::HashSet;
use std::panic;
use std::sync::Arc;

use super::{AstContext, DirAnalyzedContext, QueryContext};
use destack_ast as ast;
use destack_dir::{self as dir, LocalNodeIdAny};
use destack_source::{EnclosingSpan, File, FileId, Span};
use destack_workspace::{Module, Session};

/// Get a module by FileId.
pub fn get_module_by_file_id(session: &Session, file_id: FileId) -> Option<Arc<Module>> {
    session.modules.get_by_file_id(file_id)
}

/// Get the span of a DIR node by mapping through AST source map.
pub fn get_dir_node_span(
    ast: AstContext<'_>,
    dir: DirAnalyzedContext<'_>,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    get_node_tree_span(ast, dir.tree(), dir_node_id)
}

/// Get the span of a DIR node using one DIR node tree.
pub fn get_node_tree_span(
    ast: AstContext<'_>,
    dir_tree: &dir::NodeTree,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);

    // get the span from AST source map
    Some(ast.source_map().get(ast_node_id))
}

/// Get the main span of a DIR node (e.g., identifier for declarations).
/// Falls back to full span if no main span is set.
pub fn get_dir_node_main_span(
    ast: AstContext<'_>,
    dir: DirAnalyzedContext<'_>,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    get_node_tree_main_span(ast, dir.tree(), dir_node_id)
}

/// Get the main span of a DIR node using one DIR node tree.
pub fn get_node_tree_main_span(
    ast: AstContext<'_>,
    dir_tree: &dir::NodeTree,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);

    // try to get the main span first (e.g., identifier span for declarations)
    Some(ast.source_map().get_main_or_enclosing(ast_node_id))
}

/// Resolve the span for a DIR node within a query context.
pub(crate) fn span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Span {
    let ast = ctx.ast_context();

    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ast.source_map().get(source_id);

    Span::new(ast.file_id, ast_span.start, ast_span.end)
}

/// Resolve the main span for a DIR node when available
pub(crate) fn main_span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Option<Span> {
    let ast = ctx.ast_context();

    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ast.source_map().get_main(source_id)?;

    Some(Span::new(ast.file_id, ast_span.start, ast_span.end))
}

/// Resolve the main or enclosing span for a DIR node
pub fn main_or_enclosing_span_for_dir_node(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: LocalNodeIdAny,
) -> Span {
    let ast = ctx.ast_context();

    // resolve the source span for the node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ast.source_map().get_main_or_enclosing(source_id);

    Span::new(ast.file_id, ast_span.start, ast_span.end)
}

/// Resolve the span for a DIR node and guard against panics
pub(crate) fn span_for_dir_node_safe(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    node_id: u32,
) -> Option<Span> {
    let ast = ctx.ast_context();

    // resolve the ast node id and guard source map access
    let ast_node_id = dir_tree.get_source(node_id);
    let full_span = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        ast.source_map().get(ast_node_id)
    }));
    let Ok(full_span) = full_span else {
        return None;
    };

    Some(Span::new(ast.file_id, full_span.start, full_span.end))
}

/// Check whether a span fully contains another span
pub(crate) fn span_contains_span(parent: Span, child: Span) -> bool {
    parent.start <= child.start && parent.end >= child.end
}

/// Resolve the line start for the given offset.
pub(crate) fn line_start_for_offset(source: &str, offset: usize) -> usize {
    // clamp offset within the source bounds
    let offset = offset.min(source.len());
    let before = &source[..offset];
    before.rfind('\n').map(|idx| idx + 1).unwrap_or(0)
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
    let ast = ctx.ast_context();

    // collect enclosing spans from the source map
    let mut enclosing = ast.source_map().get_enclosing_spans(start, end);

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Collect enclosing spans at the cursor and previous byte.
pub(crate) fn enclosing_spans_with_previous(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    let ast = ctx.ast_context();

    // collect enclosing spans at the cursor position
    let mut enclosing = ast.source_map().get_enclosing_spans(offset, offset);

    // include enclosing spans at the previous byte for boundary cases
    if offset > 0 {
        let previous_offset = offset - 1;
        let mut previous = ast
            .source_map()
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

/// Find the span for a string literal matching the provided text inside an enclosing span.
pub(crate) fn string_literal_span_in_enclosing(
    file: &File,
    tokens: &[ast::TokenSpan],
    enclosing: Span,
    target_text: &str,
) -> Option<Span> {
    for token in tokens {
        if token.span.file != enclosing.file {
            continue;
        }
        if token.span.start < enclosing.start || token.span.end > enclosing.end {
            continue;
        }
        if token.token.ty != ast::TokenType::Literal {
            continue;
        }
        if !matches!(token.token.literal, Some(ast::LiteralType::String { .. })) {
            continue;
        }

        let literal = file.span_str(token.span);
        let value = string_literal_value(literal)?;
        if value == target_text {
            return Some(token.span);
        }
    }

    None
}

/// Extract the string literal contents without quotes.
fn string_literal_value(literal: &str) -> Option<&str> {
    let bytes = literal.as_bytes();
    if bytes.len() < 2 {
        return None;
    }

    let quote = match bytes[0] {
        b'"' | b'\'' => bytes[0],
        _ => return None,
    };
    if bytes[bytes.len() - 1] != quote {
        return None;
    }

    Some(&literal[1..literal.len() - 1])
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
