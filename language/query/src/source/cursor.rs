use destack_dir as dir;
use destack_source::{EnclosingSpan, NodeSpanRegion, NodeSpanType, Span};

use super::{
    DirQueryContext, enclosing_spans_at_offsets, enclosing_spans_with_previous,
    next_significant_token, previous_significant_token,
};

/// Collect probe offsets that should share one cursor owner.
fn cursor_probe_offsets(ctx: DirQueryContext<'_>, offset: u32) -> Vec<u32> {
    let mut offsets = vec![offset];

    // include the previous byte for ordinary boundary cases
    if offset > 0 {
        offsets.push(offset - 1);
    }

    // include the previous significant token interior when the cursor trails it
    if let Some(token) = previous_significant_token(ctx, offset)
        && token.span.end <= offset
        && token.span.start < token.span.end
    {
        offsets.push(token.span.end - 1);
    }

    // include the next significant token start when the cursor precedes it
    if let Some(token) = next_significant_token(ctx, offset)
        && token.span.start >= offset
    {
        offsets.push(token.span.start);
    }

    offsets
}

/// Collect enclosing spans around a cursor.
///
/// This supplements the direct cursor probes with adjacent significant token probes
/// so recovered source ownership survives when the cursor sits in whitespace gaps.
pub(crate) fn enclosing_spans_at_cursor(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    let offsets = cursor_probe_offsets(ctx, offset);

    enclosing_spans_at_offsets(ctx, offsets)
}

/// Check whether one span owns the cursor.
pub(crate) fn span_owns_cursor(span: Span, offset: u32) -> bool {
    // the span directly owns the cursor
    if span.contains(offset) {
        return true;
    }

    // boundary ownership also includes the previous byte
    let previous_offset = offset.saturating_sub(1);
    if span.contains(previous_offset) {
        return true;
    }

    span.start == offset && span.end == offset
}

/// Check whether an offset falls inside one source type side span.
pub(crate) fn offset_is_in_source_type_side_span(ctx: DirQueryContext<'_>, offset: u32) -> bool {
    let previous_offset = offset.saturating_sub(1);

    for enclosing in enclosing_spans_with_previous(ctx, offset) {
        // type side spans own both the cursor and the previous byte boundary
        if let Some(span) = ctx
            .source_map()
            .get_side(enclosing.idx, NodeSpanType::Region(NodeSpanRegion::Type))
            && (span.contains(offset) || span.contains(previous_offset))
        {
            return true;
        }
    }

    false
}

/// Resolve the innermost missing expression at the cursor.
pub(crate) fn enclosing_missing_expression(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // walk inward to outward until one missing expression claims the cursor
    for enclosing in enclosing_spans_at_cursor(ctx, offset) {
        if ctx.tree().get_node_type(enclosing.idx) != dir::NodeType::Expression {
            continue;
        }

        let expr_id = dir::LocalNodeId::<dir::Expression>::new(enclosing.idx);

        // only recovered missing expressions classify expression slots
        if matches!(ctx.tree().get(expr_id), dir::Expression::Missing) {
            return Some(expr_id);
        }
    }

    None
}
