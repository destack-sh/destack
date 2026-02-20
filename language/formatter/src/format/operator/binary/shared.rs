use crate::analysis::scan::next_non_whitespace_after_span;
use crate::operator::{
    Annotation, AnnotationPosition, BinaryOperator, DestackFormatContext, DestackFormatter,
    Expression, FormatResult, LocalNodeId, NodeType, ParenthesizedDropPolicy,
    expression_has_leading_prefix_comment,
    expression_has_non_doc_multiline_block_prefix_comment_annotation,
    has_comment_between_expressions, parenthesized_should_drop, space, span_has_comment,
};
use destack_fir::prelude::*;
use destack_fir::write;

/// Return whether a leading type union has an expression ancestor with block-prefix comments.
pub(super) fn leading_union_has_ancestor_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;
    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let has_block_prefix_comment =
            expression_has_non_doc_multiline_block_prefix_comment_annotation(
                context,
                parent_expression_id,
            );
        if has_block_prefix_comment {
            return true;
        }

        current_id = parent_expression_id;
    }

    false
}

/// Return whether an expression is directly wrapped by a parenthesized expression.
pub(super) fn expression_parent_is_parenthesized(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id)
            else {
                return false;
            };
            if *expression != node_id {
                return false;
            }

            !parenthesized_should_drop(
                context,
                parent_expression_id,
                node_id,
                ParenthesizedDropPolicy::ExpressionWrapper,
            )
        })
}

/// Return whether one binary operator is logical.
#[inline]
pub(super) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether an expression ends with a `//` postfix annotation.
pub(super) fn expression_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };

        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether an expression starts with a `//` line-prefix annotation.
pub(super) fn expression_has_line_prefix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if position != AnnotationPosition::LinePrefix {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether an expression starts with an inline `/* ... */` prefix annotation.
pub(super) fn expression_has_inline_block_prefix_star_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != destack_ast::CommentStyle::Star {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        !context.has_newline(annotation_span)
    })
}

/// Return whether root prefix comments should keep the first leading `|` inline.
pub(super) fn leading_union_root_prefers_inline_first_pipe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    let mut candidate: Option<LocalNodeId<Annotation>> = None;
    for annotation_id in annotation_ids {
        let Annotation::Comment {
            position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
            ..
        } = context.annotation(annotation_id)
        else {
            continue;
        };

        let is_before_type_grouping_operator = matches!(
            next_non_whitespace_after_span(context, context.annotation_span(annotation_id)),
            Some('|' | '&')
        );
        if !is_before_type_grouping_operator {
            continue;
        }

        candidate = Some(annotation_id);
    }

    let Some(annotation_id) = candidate else {
        return false;
    };

    context.span_starts_on_own_line(context.annotation_span(annotation_id))
}

/// Return whether mixed logical precedence should parenthesize the right expression.
#[inline]
pub(super) fn is_mixed_logical_precedence_pair(
    left_operator: BinaryOperator,
    right_operator: BinaryOperator,
) -> bool {
    matches!(left_operator, BinaryOperator::Or | BinaryOperator::Coalesce)
        && left_operator != right_operator
        && matches!(
            right_operator,
            BinaryOperator::And | BinaryOperator::Coalesce
        )
}

/// Return whether a mixed logical precedence pair should preserve right grouping for comments.
#[inline]
pub(super) fn should_preserve_mixed_logical_grouping_for_comments(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right_span = context.span(right);
    span_has_comment(context, right_span) || has_comment_between_expressions(context, left, right)
}

/// Return whether a source operator break should be preserved.
#[inline]
pub(super) fn preserve_source_operator_break(
    operator: BinaryOperator,
    has_source_operator_break: bool,
) -> bool {
    !is_logical_binary_operator(operator) && has_source_operator_break
}

/// Return whether a logical operand prefers trailing-operator layout.
#[inline]
pub(super) fn operand_prefers_trailing_logical_operator(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator)
        && expression_has_leading_prefix_comment(context, operand_expression)
}

/// Return whether a logical binary left operand ends with a line postfix slash comment.
pub(super) fn logical_left_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    left: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator) && expression_has_line_postfix_slash_comment(context, left)
}

/// Write one separating space after the left operand when no postfix trivia exists.
pub(super) fn write_space_after_binary_left_if_needed<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> FormatResult<()> {
    let allow_logical_space_after_line_comment =
        logical_left_has_line_postfix_slash_comment(f.context(), operator, left);
    if f.context().has_postfix_annotation(left) && !allow_logical_space_after_line_comment {
        return Ok(());
    }

    write!(f, [space()])
}
