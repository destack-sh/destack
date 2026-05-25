use super::DirQueryContext;
use destack_dir as dir;
use destack_dir::LocalNodeIdAny;
use destack_source::{EnclosingSpan, File, Span};
use std::collections::HashSet;

/// Get the span of a DIR node using one DIR tree.
pub(crate) fn get_node_tree_span(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    dir_node_id: LocalNodeIdAny,
) -> Span {
    // get the source node id from the DIR node
    let source_node_id = dir.get_source_any(dir_node_id);

    // get the span from source index
    ctx.source_index().get(source_node_id)
}

/// Get the main span of a DIR node using one DIR tree.
pub(crate) fn get_node_tree_main_span(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    dir_node_id: LocalNodeIdAny,
) -> Span {
    // get the source node id from the DIR node
    let source_node_id = dir.get_source_any(dir_node_id);

    // try to get the main span first (e.g., identifier span for declarations)
    ctx.source_index().get_main_or_enclosing(source_node_id)
}

/// Resolve the span for a DIR node within a query context.
pub(crate) fn span_for_dir_node(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    node_id: LocalNodeIdAny,
) -> Span {
    // resolve the source span for the node
    let source_id = dir.get_source_any(node_id);
    let source_span = ctx.source_index().get(source_id);

    Span::new(ctx.file_id(), source_span.start, source_span.end)
}

/// Resolve the span for a DIR node when its source id is present in the source index.
pub(crate) fn try_span_for_dir_node(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    node_id: LocalNodeIdAny,
) -> Option<Span> {
    // resolve the source span when the source id is still valid
    let source_id = dir.get_source_any(node_id);
    let source_span = ctx.source_index().try_get(source_id)?;

    Some(Span::new(ctx.file_id(), source_span.start, source_span.end))
}

/// Resolve the main span for a DIR node when available.
pub(crate) fn main_span_for_dir_node(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    node_id: LocalNodeIdAny,
) -> Option<Span> {
    // resolve the source span for the node
    let source_id = dir.get_source_any(node_id);
    let source_span = ctx.source_index().get_main(source_id)?;

    Some(Span::new(ctx.file_id(), source_span.start, source_span.end))
}

/// Resolve the main or enclosing span for a DIR node.
pub(crate) fn main_or_enclosing_span_for_dir_node(
    ctx: DirQueryContext<'_>,
    dir: dir::View<'_>,
    node_id: LocalNodeIdAny,
) -> Span {
    // resolve the source span for the node
    let source_id = dir.get_source_any(node_id);
    let source_span = ctx.source_index().get_main_or_enclosing(source_id);

    Span::new(ctx.file_id(), source_span.start, source_span.end)
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
    ctx: DirQueryContext<'_>,
    start: u32,
    end: u32,
) -> Vec<EnclosingSpan> {
    // collect enclosing spans from the source index
    let mut enclosing = ctx.source_index().get_enclosing_spans(start, end);

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Collect and sort enclosing spans for a set of probe offsets.
pub(crate) fn enclosing_spans_at_offsets(
    ctx: DirQueryContext<'_>,
    offsets: impl IntoIterator<Item = u32>,
) -> Vec<EnclosingSpan> {
    let mut enclosing = Vec::new();
    let mut seen = HashSet::new();

    // gather the enclosing spans for each probe offset
    for offset in offsets {
        let spans = ctx.source_index().get_enclosing_spans(offset, offset);
        for span in spans {
            if seen.insert(span.idx) {
                enclosing.push(span);
            }
        }
    }

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Collect enclosing spans at the cursor and previous byte.
pub(crate) fn enclosing_spans_with_previous(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    let mut offsets = vec![offset];
    if offset > 0 {
        offsets.push(offset - 1);
    }

    enclosing_spans_at_offsets(ctx, offsets)
}
/// Find the span for a string literal matching the provided text inside an enclosing span.
pub(crate) fn string_literal_span_in_enclosing(
    file: &File,
    tokens: &[dir::TokenSpan],
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
        if token.token.ty() != dir::TokenType::Literal {
            continue;
        }
        if !matches!(
            token.token.literal(),
            Some(dir::TokenLiteral::String { .. })
        ) {
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
