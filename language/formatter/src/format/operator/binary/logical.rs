use super::super::common::expression_is_trivial_inline_without_annotations;
use super::shared::{
    expression_has_inline_block_prefix_star_comment, expression_has_line_postfix_slash_comment,
    expression_has_line_prefix_slash_comment, expression_parent_is_parenthesized,
    is_logical_binary_operator, is_mixed_logical_precedence_pair,
    leading_union_has_ancestor_block_prefix_annotation,
    leading_union_root_prefers_inline_first_pipe,
    should_preserve_mixed_logical_grouping_for_comments, write_space_after_binary_left_if_needed,
};
use super::type_layout::type_binary_operands_are_structurally_complex;
use crate::Annotation;
use crate::analysis::scan::last_non_trivia_token_in_span;
use crate::chain::{BinaryOperands, flatten_binary_expression, should_use_trailing_coalesce};
use crate::expression::{
    BinaryOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    format_with, group, hard_line_break, indent, soft_line_break_or_space, space, token,
};
use crate::operator::{
    format_binary_operand_without_prefix_annotations_with_grouping_parentheses,
    is_object_like_type_expression, is_static_type_argument_context,
    should_hug_nullable_union_type, union_source_has_leading_pipe,
};
use destack_ast::TokenType;
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Try formatting `??` using trailing-operator layout.
pub(super) fn try_format_trailing_coalesce<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if operator != BinaryOperator::Coalesce
        || !should_use_trailing_coalesce(f.context(), node_id, left)
    {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            indent(&format_with(|f| {
                write_space_after_binary_left_if_needed(f, left, operator)?;
                write!(f, [operator, soft_line_break_or_space(), right])
            }))
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical operators with a right-side line-prefix comment seam.
pub(super) fn try_format_logical_right_prefix_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }
    if !expression_has_line_prefix_slash_comment(f.context(), right) {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            space(),
            indent(&format_args![right])
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical operators with an inline right-side block-prefix comment seam.
pub(super) fn try_format_logical_right_prefix_block_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }
    if !expression_has_inline_block_prefix_star_comment(f.context(), right) {
        return Ok(false);
    }
    if flatten_binary_expression(f.context().tree, node_id, operator).len() > 2 {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            indent(&format_args![soft_line_break_or_space(), right])
        ])]
    )?;

    Ok(true)
}

/// Try formatting mixed logical precedence pairs with explicit right parentheses.
pub(super) fn try_format_mixed_logical_precedence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Expression::Binary {
        operator: right_operator,
        ..
    } = f.context().tree.get(right)
    else {
        return Ok(false);
    };
    if !is_mixed_logical_precedence_pair(operator, *right_operator) {
        return Ok(false);
    }

    if !should_preserve_mixed_logical_grouping_for_comments(f.context(), left, right) {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            space(),
            token("("),
            right,
            token(")")
        ])]
    )?;

    Ok(true)
}

/// Try formatting logical expressions with parenthesized-tail policies.
pub(super) fn try_format_logical_parenthesized_cases<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }

    let left_span = f.context().span(left);
    let left_has_multiline_parenthesized_tail = f.context().has_newline(left_span)
        && last_non_trivia_token_in_span(f.context(), left_span)
            .is_some_and(|token| token.token.ty == TokenType::CloseParenthesis);
    let right_expression = f.context().tree.get(right);
    let right_is_inline_trivial =
        expression_is_trivial_inline_without_annotations(f.context(), right);
    let right_has_prefix = f.context().has_prefix_annotation(right);

    // parenthesized multiline left tail with short `&&` right side
    if left_has_multiline_parenthesized_tail
        && right_is_inline_trivial
        && !right_has_prefix
        && operator == BinaryOperator::And
    {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
                operator,
                indent(&format_args![hard_line_break(), right])
            ])]
        )?;
        return Ok(true);
    }

    // prefix-commented left parentheses keep trailing logical operators
    let left_prefers_trailing_operator = matches!(
        f.context().tree.get(left),
        Expression::Parenthesized { expression }
            if f.context().has_prefix_annotation(left)
                || f.context().has_prefix_annotation(*expression)
    );
    if left_prefers_trailing_operator && right_is_inline_trivial {
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(true);
    }

    // keep tree rhs inline when the rhs is parenthesized tree
    let is_parenthesized_tree = matches!(
        right_expression,
        Expression::Parenthesized { expression }
            if matches!(f.context().tree.get(*expression), Expression::TreeExpression { .. })
    );
    if !is_parenthesized_tree {
        return Ok(false);
    }

    write!(
        f,
        [group(&format_args![
            left,
            space(),
            operator,
            space(),
            right
        ])]
    )?;

    Ok(true)
}

/// Store leading-pipe union render policy flags.
#[derive(Clone, Copy)]
struct LeadingPipeUnionLayoutPolicy {
    should_indent_operands: bool,
    prefer_space_before_first_pipe: bool,
}

/// Return whether a union should use leading-pipe multiline style.
pub(super) fn should_use_leading_pipe_union_style(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
) -> bool {
    let is_static_type_argument = is_static_type_argument_context(context, node_id);
    let is_template_literal_interpolation =
        context.expression_is_in_template_literal_interpolation(node_id);
    let has_structural_complexity =
        type_binary_operands_are_structurally_complex(context, operands);
    let has_source_leading_pipe = union_source_has_leading_pipe(context, node_id);
    if is_template_literal_interpolation && !has_source_leading_pipe {
        return false;
    }

    let has_layout_forcing_comments =
        type_binary_operands_have_layout_forcing_comments(context, operands);
    let has_many_union_operands = operands.len() > 2;
    let union_has_newline = context.node_has_newline(node_id);
    let nullable_object_union_prefers_leading_pipe =
        should_hug_nullable_union_type(context, operands)
            && operands
                .iter()
                .any(|operand| is_object_like_type_expression(context, operand.expression));
    let last_operand_has_line_postfix_comment = operands.last().is_some_and(|operand| {
        expression_has_line_postfix_slash_comment(context, operand.expression)
    });

    if has_layout_forcing_comments {
        return true;
    }

    if is_static_type_argument && !has_source_leading_pipe {
        return false;
    }

    if has_source_leading_pipe {
        return union_has_newline && !last_operand_has_line_postfix_comment
            || has_many_union_operands
            || has_structural_complexity
            || nullable_object_union_prefers_leading_pipe;
    }

    if has_many_union_operands {
        return true;
    }

    if nullable_object_union_prefers_leading_pipe {
        return union_has_newline || has_structural_complexity;
    }

    has_structural_complexity && union_has_newline
}

/// Return whether type binary operands contain non-doc comments that force multiline layout.
fn type_binary_operands_have_layout_forcing_comments(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    operands.iter().enumerate().any(|(index, operand)| {
        let Some(annotation_ids) = context.annotations(operand.expression) else {
            return false;
        };

        annotation_ids.into_iter().any(|annotation_id| {
            let Annotation::Comment { node, .. } = context.annotation(annotation_id) else {
                return false;
            };

            let comment = context.tree.get::<destack_ast::Comment>(node);
            let is_last_operand = index + 1 == operands.len();
            if is_last_operand && comment.style == destack_ast::CommentStyle::Slash {
                return false;
            }
            true
        })
    })
}

/// Build leading-pipe union render policy from current annotation context.
fn leading_pipe_union_layout_policy(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LeadingPipeUnionLayoutPolicy {
    let has_block_prefix_ancestor =
        leading_union_has_ancestor_block_prefix_annotation(context, node_id);
    let root_prefers_inline_first_pipe =
        leading_union_root_prefers_inline_first_pipe(context, node_id);
    let is_parenthesized_union = expression_parent_is_parenthesized(context, node_id);

    // root prefix annotations already place the union under a newline-aware seam
    // extra indent here can double-indent operands and separate trailing semicolons
    // parenthesized unions already receive indent from the wrapper
    let should_indent_operands =
        !has_block_prefix_ancestor && !root_prefers_inline_first_pipe && !is_parenthesized_union;

    LeadingPipeUnionLayoutPolicy {
        should_indent_operands,
        prefer_space_before_first_pipe: has_block_prefix_ancestor || root_prefers_inline_first_pipe,
    }
}

/// Write the first operand of a leading-pipe union.
fn write_first_leading_pipe_union_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    policy: LeadingPipeUnionLayoutPolicy,
    expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_prefix_annotation(expression) {
        if policy.should_indent_operands {
            write!(
                f,
                [indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        write!(f, [f.context().any_prefix_annotations(expression)])?;
                        if policy.prefer_space_before_first_pipe {
                            write!(f, [token("|"), space()])?;
                        } else {
                            write!(f, [soft_line_break_or_space(), token("|"), space()])?;
                        }

                        format_binary_operand_without_prefix_annotations_with_grouping_parentheses(
                            f,
                            BinaryOperator::ElementwiseOr,
                            expression,
                        )
                    }
                ))]
            )?;
            return Ok(());
        }

        write!(f, [f.context().any_prefix_annotations(expression)])?;
        if policy.prefer_space_before_first_pipe {
            write!(f, [token("|"), space()])?;
        } else {
            write!(f, [soft_line_break_or_space(), token("|"), space()])?;
        }

        return format_binary_operand_without_prefix_annotations_with_grouping_parentheses(
            f,
            BinaryOperator::ElementwiseOr,
            expression,
        );
    }

    if policy.should_indent_operands {
        write!(
            f,
            [indent(&format_with(|f| {
                if policy.prefer_space_before_first_pipe {
                    write!(f, [token("|"), space()])?;
                } else {
                    write!(f, [soft_line_break_or_space(), token("|"), space()])?;
                }
                write_leading_pipe_union_operand_expression(f, expression)
            }))]
        )?;
        return Ok(());
    }

    if policy.prefer_space_before_first_pipe {
        write!(f, [token("|"), space()])?;
    } else {
        write!(f, [soft_line_break_or_space(), token("|"), space()])?;
    }

    write_leading_pipe_union_operand_expression(f, expression)
}

/// Write one non-first operand in a leading-pipe union.
fn write_trailing_leading_pipe_union_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    policy: LeadingPipeUnionLayoutPolicy,
    prev_expression: Option<LocalNodeId<Expression>>,
    operand_expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let has_postfix = prev_expression
        .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
    let has_line_postfix_slash_comment = prev_expression.is_some_and(|expression_id| {
        expression_has_line_postfix_slash_comment(f.context(), expression_id)
    });

    if policy.should_indent_operands {
        write!(
            f,
            [indent(&format_with(|f| {
                if has_line_postfix_slash_comment || has_postfix {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }
                write!(f, [token("|"), space()])?;
                write_leading_pipe_union_operand_expression(f, operand_expression)
            }))]
        )?;
        return Ok(());
    }

    if has_line_postfix_slash_comment || has_postfix {
        write!(f, [hard_line_break()])?;
    } else {
        write!(f, [soft_line_break_or_space()])?;
    }
    write!(f, [token("|"), space()])?;
    write_leading_pipe_union_operand_expression(f, operand_expression)
}

/// Write one leading-pipe union operand expression with nested indentation for object-like arms.
fn write_leading_pipe_union_operand_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_object_like_type_expression(f.context(), expression) {
        write!(f, [indent(&format_with(|f| write!(f, [expression])))])
    } else {
        write!(f, [expression])
    }
}

/// Format leading-pipe union operands with shared annotation and comment policy.
pub(super) fn format_leading_pipe_union<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    let policy = leading_pipe_union_layout_policy(f.context(), node_id);
    let is_parenthesized_union = expression_parent_is_parenthesized(f.context(), node_id);

    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut prev_expression: Option<LocalNodeId<Expression>> = None;
            for (index, operand) in operands.iter().enumerate() {
                if index == 0 {
                    write_first_leading_pipe_union_operand(f, policy, operand.expression)?;
                } else {
                    write_trailing_leading_pipe_union_operand(
                        f,
                        policy,
                        prev_expression,
                        operand.expression,
                    )?;
                }

                prev_expression = Some(operand.expression);
            }

            Ok(())
        }))
        .should_expand(true)]
    )?;

    // keep multiline parenthesized unions with a closing delimiter on its own line
    if is_parenthesized_union {
        write!(f, [hard_line_break()])?;
    }

    Ok(())
}
