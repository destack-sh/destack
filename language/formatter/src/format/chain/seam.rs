use super::{DestackFormatContext, Expression, LocalNodeId, Span, span_has_comment};

/// Check whether source contains a newline between two expression nodes.
pub(crate) fn has_newline_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    context.has_newline(Span::new(left_span.file, left_span.end, right_span.start))
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    span_has_comment(
        context,
        Span::new(left_span.file, left_span.end, right_span.start),
    )
}

/// Return whether a `//` comment exists between two expression nodes.
pub(crate) fn has_line_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    context
        .line_comment_spans
        .iter()
        .copied()
        .any(|comment_span| {
            comment_span.file == left_span.file
                && comment_span.start >= left_span.end
                && comment_span.end <= right_span.start
        })
}
