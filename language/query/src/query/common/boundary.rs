use destack_ast as ast;
use destack_source::{EnclosingSpan, NodeSpanType, Span};

use super::{
    QueryContext, enclosing_spans_at_offsets, enclosing_spans_with_previous,
    next_significant_token, previous_significant_token,
};

/// Collect probe offsets that should share one cursor boundary owner.
fn cursor_boundary_probe_offsets(ctx: &QueryContext<'_>, offset: u32) -> Vec<u32> {
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

/// Collect enclosing spans around a cursor boundary.
///
/// This supplements the direct cursor probes with adjacent significant token probes
/// so recovered syntax ownership survives when the cursor sits in whitespace gaps.
pub(crate) fn enclosing_spans_at_cursor_boundary(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Vec<EnclosingSpan> {
    let offsets = cursor_boundary_probe_offsets(ctx, offset);

    enclosing_spans_at_offsets(ctx.ast_context(), offsets)
}

/// Check whether one span owns the cursor boundary.
pub(crate) fn span_owns_cursor_boundary(span: Span, offset: u32) -> bool {
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

/// Check whether an offset falls inside one AST type side span.
pub(crate) fn offset_is_in_ast_type_side_span(ctx: &QueryContext<'_>, offset: u32) -> bool {
    let previous_offset = offset.saturating_sub(1);

    for enclosing in enclosing_spans_with_previous(ctx, offset) {
        // type side spans own both the cursor and the previous byte boundary
        if let Some(span) = ctx
            .ast_context()
            .source_map()
            .get_side(enclosing.idx, NodeSpanType::Type)
            && (span.contains(offset) || span.contains(previous_offset))
        {
            return true;
        }
    }

    false
}

/// Resolve the innermost missing expression at the cursor boundary.
pub(crate) fn enclosing_missing_expression(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // walk inward to outward until one missing expression claims the cursor
    for enclosing in enclosing_spans_at_cursor_boundary(ctx, offset) {
        if ctx.ast_context().tree().get_node_type(enclosing.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enclosing.idx);

        // only recovered missing expressions classify expression slots
        if matches!(
            ctx.ast_context().tree().get(expr_id),
            ast::Expression::Missing
        ) {
            return Some(expr_id);
        }
    }

    None
}
