use crate::DestackFormatContext;
use crate::analysis::scan::next_non_whitespace_after_span;
use crate::analysis::timing::tags;
use crate::expression::span_has_comment;
use destack_ast::{Comment, CommentStyle, Expression, LocalNodeId};
use destack_source::Span;

/// Collect postfix star comments from an inner expression that should render after `)`.
pub(crate) fn collect_parenthesized_boundary_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    let _timing =
        context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_BOUNDARY_COMMENTS);
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return Vec::new();
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return Vec::new();
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);
    if parenthesized_span.file != inner_span.file || inner_span.end >= parenthesized_span.end {
        return Vec::new();
    }

    let boundary_span = Span::new(
        parenthesized_span.file,
        inner_span.end,
        parenthesized_span.end,
    );
    if !context.has_comment(boundary_span) {
        return Vec::new();
    }

    let comment_trivia = context.tree.comment_trivia();
    let first_relevant_index =
        comment_trivia.partition_point(|comment_trivia| comment_trivia.span.end < inner_span.end);

    let mut comments: Vec<(u32, LocalNodeId<Comment>)> = Vec::new();
    for comment_trivia in comment_trivia[first_relevant_index..].iter().copied() {
        if comment_trivia.span.start > parenthesized_span.end {
            break;
        }
        if context.tree.get(comment_trivia.comment).style != CommentStyle::Star {
            continue;
        }
        if comment_trivia.span.start < inner_span.end
            || comment_trivia.span.end > parenthesized_span.end
        {
            continue;
        }

        if next_non_whitespace_after_span(context, comment_trivia.span) != Some(')') {
            continue;
        }

        comments.push((comment_trivia.span.start, comment_trivia.comment));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment_id)| comment_id)
        .collect()
}

/// Return whether source contains leading trivia between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, true)
}

/// Return whether source contains leading comments between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, false)
}

/// Return whether source contains a newline between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_newline(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    context.has_newline(leading_span)
}

/// Return whether source contains leading comment or newline trivia between `(` and inner.
fn parenthesized_has_leading_inner_pattern(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    include_newline: bool,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_LEADING_TRIVIA);
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    if include_newline && context.has_newline(leading_span) {
        return true;
    }

    span_has_comment(context, leading_span)
}
