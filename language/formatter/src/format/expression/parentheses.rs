use super::{
    argument_drops_parenthesized_value_wrapper, declarator_drops_parenthesized_value_wrapper,
    postfix_continuation_requires_parenthesized_object_wrapper,
    write_expression_with_prefix_annotations_after_offset,
};
use crate::format::annotation::{write_raw_comment_slice, write_raw_leading_comments};
use crate::format::call::call_drops_parenthesized_callee_wrapper;
use crate::format::chain::transparent_inner_expression;
use crate::format::context::ParenthesizedExpressionView;
use crate::format::operator::{
    assignment_drops_parenthesized_operand_wrapper, binary_keeps_unary_left_parenthesized_wrapper,
    format_binary_expression, should_drop_parenthesized_type_expression,
};
use crate::format::tree::format_parenthesized_tree_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{BinaryOperator, Comment, Expression, LocalNodeId, NodeType};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, soft_block_indent, token,
};
use destack_fir::write;

/// Collect comments that belong immediately before one closing `)`.
fn parenthesized_trailing_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    _inner_expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    ParenthesizedExpressionView::from_node(context, parenthesized_id)
        .map(ParenthesizedExpressionView::trailing_inner_comments)
        .unwrap_or_default()
}

/// Return closing-paren comments that belong to the parenthesized shell.
fn formatter_owned_parenthesized_trailing_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    parenthesized_trailing_inner_comments(context, parenthesized_id, inner_expression_id)
}

/// Collect comments between `(` and the inner expression.
pub(crate) fn parenthesized_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    _inner_expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    ParenthesizedExpressionView::from_node(context, parenthesized_id)
        .map(ParenthesizedExpressionView::leading_inner_comments)
        .unwrap_or_default()
}

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
pub(crate) fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_view = ParenthesizedExpressionView::from_node(context, node_id);
    if parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia) {
        return false;
    }

    let should_drop_type_parentheses =
        should_drop_parenthesized_type_expression(context, node_id, inner_expression_id);
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return should_drop_type_parentheses;
    };

    // non-expression parents use structural argument or declarator wrapper rules
    if parent_type != NodeType::Expression {
        let should_drop_argument_wrapper = parent_type == NodeType::Argument
            && argument_drops_parenthesized_value_wrapper(context, node_id, inner_expression_id);
        let should_drop_declarator_wrapper = parent_type == NodeType::Declarator
            && declarator_drops_parenthesized_value_wrapper(context, node_id, inner_expression_id);

        if should_drop_argument_wrapper || should_drop_declarator_wrapper {
            return true;
        }

        return should_drop_type_parentheses;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_expression_id);
    let inner_expression = context.tree.get(inner_expression_id);

    if postfix_continuation_requires_parenthesized_object_wrapper(
        context,
        node_id,
        parent_expression,
        inner_expression_id,
    ) {
        return false;
    }

    // binary owner
    if binary_keeps_unary_left_parenthesized_wrapper(node_id, inner_expression, parent_expression) {
        return false;
    }

    let should_drop_call_callee_instantiation_wrapper = call_drops_parenthesized_callee_wrapper(
        context,
        node_id,
        inner_expression_id,
        parent_expression,
    );
    let should_drop_assignment_wrapper = assignment_drops_parenthesized_operand_wrapper(
        context,
        node_id,
        inner_expression_id,
        parent_expression,
    );
    should_drop_assignment_wrapper
        || should_drop_call_callee_instantiation_wrapper
        || should_drop_type_parentheses
}

/// Format one dropped parenthesized expression wrapper.
fn format_dropped_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let normalized_inner_id = transparent_inner_expression(f.context(), expression_id);
    if let Expression::Binary {
        left,
        operator: operator @ (BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd),
        right,
    } = f.context().tree.get(normalized_inner_id)
    {
        return format_binary_expression(f, normalized_inner_id, *left, operator, *right);
    }

    write!(f, [expression_id])
}

/// Format one preserved wrapper with explicit leading inner trivia.
fn format_leading_trivia_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let parenthesized_view = ParenthesizedExpressionView::from_node(f.context(), node_id);
    let leading_inner_comments =
        parenthesized_leading_inner_comments(f.context(), node_id, expression_id);
    let has_leading_inner_newline =
        parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_newline);
    let trailing_inner_comment_nodes =
        formatter_owned_parenthesized_trailing_inner_comments(f.context(), node_id, expression_id);
    let leading_inner_end = parenthesized_view
        .and_then(ParenthesizedExpressionView::leading_inner_span)
        .map_or(f.context().span(expression_id).start, |span| span.end);
    let can_keep_inline = !has_leading_inner_newline
        && leading_inner_comments.iter().copied().all(|comment| {
            comment.is_block()
                && !f.context().has_newline(comment.span)
                && !f.context().span_starts_on_own_line(comment.span)
                && !f
                    .context()
                    .span_has_newline_before_next_non_whitespace_token(comment.span)
        })
        && trailing_inner_comment_nodes.iter().copied().all(|comment| {
            comment.is_block()
                && !f.context().has_newline(comment.span)
                && !f.context().span_starts_on_own_line(comment.span)
                && !f
                    .context()
                    .span_has_newline_before_next_non_whitespace_token(comment.span)
        });

    if can_keep_inline {
        write!(f, [token("(")])?;

        if !leading_inner_comments.is_empty() {
            write_raw_leading_comments(f, &leading_inner_comments)?;
        }

        write_expression_with_prefix_annotations_after_offset(f, expression_id, leading_inner_end)?;

        if !trailing_inner_comment_nodes.is_empty() {
            write!(
                f,
                [format_with(|f| write_raw_comment_slice(
                    f,
                    &trailing_inner_comment_nodes,
                ))]
            )?;
        }

        write!(f, [token(")")])?;
        return Ok(());
    }

    write!(
        f,
        [
            token("("),
            block_indent(&format_with(|f| {
                // preserve source owned comments between `(` and the inner expression
                if !leading_inner_comments.is_empty() {
                    write_raw_leading_comments(f, &leading_inner_comments)?;
                }
                // otherwise preserve the explicit wrapper newline
                else if has_leading_inner_newline {
                    write!(f, [hard_line_break()])?;
                }

                // print the inner expression through the regular expression writer
                write!(
                    f,
                    [group(&format_with(|f| {
                        write_expression_with_prefix_annotations_after_offset(
                            f,
                            expression_id,
                            leading_inner_end,
                        )
                    }))
                    .should_expand(true)]
                )?;

                // keep comments that belong immediately before the closing `)`
                if !trailing_inner_comment_nodes.is_empty() {
                    write!(
                        f,
                        [format_with(|f| write_raw_comment_slice(
                            f,
                            &trailing_inner_comment_nodes,
                        ))]
                    )?;
                }

                Ok(())
            })),
            hard_line_break(),
            token(")")
        ]
    )
}

/// Format one preserved parenthesized expression wrapper.
fn format_preserved_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let parenthesized_view = ParenthesizedExpressionView::from_node(f.context(), node_id);
    let trailing_inner_comment_nodes =
        formatter_owned_parenthesized_trailing_inner_comments(f.context(), node_id, expression_id);
    let has_leading_inner_trivia =
        parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia);

    // tree expressions keep the specialized tree formatter
    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = f.context().tree.get(expression_id)
    {
        return format_parenthesized_tree_expression(
            f,
            node_id,
            expression_id,
            arguments,
            elements,
            has_leading_inner_trivia,
            &trailing_inner_comment_nodes,
        );
    }

    // preserved leading trivia keeps the wrapper in one generic expanded form
    if has_leading_inner_trivia {
        format_leading_trivia_parenthesized_expression(f, node_id, expression_id)?;
    }
    // otherwise keep the explicit wrapper inline
    else {
        let should_expand_wrapper = trailing_inner_comment_nodes.iter().copied().any(|comment| {
            comment.is_line()
                || f.context().has_newline(comment.span)
                || f.context().span_starts_on_own_line(comment.span)
        });

        if should_expand_wrapper {
            write!(
                f,
                [
                    token("("),
                    soft_block_indent(&format_with(|f| {
                        write!(f, [expression_id])?;

                        if !trailing_inner_comment_nodes.is_empty() {
                            write!(
                                f,
                                [format_with(|f| write_raw_comment_slice(
                                    f,
                                    &trailing_inner_comment_nodes,
                                ))]
                            )?;
                        }

                        Ok(())
                    })),
                    token(")")
                ]
            )?;
        } else {
            write!(f, [token("("), expression_id])?;

            if !trailing_inner_comment_nodes.is_empty() {
                write!(
                    f,
                    [format_with(|f| write_raw_comment_slice(
                        f,
                        &trailing_inner_comment_nodes,
                    ))]
                )?;
            }

            write!(f, [token(")")])?;
        }
    }

    Ok(())
}

/// Format a parenthesized primary expression.
pub(crate) fn format_primary_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if should_drop_parenthesized_expression_wrapper(f.context(), node_id, expression_id) {
        return format_dropped_parenthesized_expression(f, node_id, expression_id);
    }

    format_preserved_parenthesized_expression(f, node_id, expression_id)
}
