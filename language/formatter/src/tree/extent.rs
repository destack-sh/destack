use crate::DestackFormatContext;
use destack_dir::{Comment, Expression, LocalNodeId};

/// Return the end of one expression source extent.
pub(crate) fn expression_source_extent_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    context.tree.get_source_extent(expression_id).end
}

/// Return line comments owned by one expression source extent after its node span.
pub(crate) fn expression_source_extent_trailing_line_comments<'a>(
    context: &'a DestackFormatContext<'a>,
    expression_id: LocalNodeId<Expression>,
) -> &'a [Comment] {
    let span = context.span(expression_id);
    let source_extent = context.tree.get_source_extent(expression_id);
    if source_extent.end <= span.end {
        return &[];
    }

    let comments = context
        .comments()
        .comments_in_range(span.end, source_extent.end);
    if comments.iter().any(|comment| {
        comment.is_line()
            || context
                .source_text()
                .contains_newline_between(span.end, comment.span.start)
    }) {
        comments
    } else {
        &[]
    }
}
