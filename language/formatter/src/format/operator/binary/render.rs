use super::logical;
use super::shared::{
    expression_has_line_postfix_slash_comment, expression_has_line_prefix_slash_comment,
    is_logical_binary_operator, logical_left_has_line_postfix_slash_comment,
    operand_prefers_trailing_logical_operator, preserve_existing_operator_break,
};
use super::type_layout::type_binary_operands_are_structurally_complex;
use crate::analysis::timing::tags;
use crate::chain::{
    flatten_binary_expression, flatten_type_binary_expression, has_newline_between_expressions,
};
use crate::expression::{
    BinaryOperator, DestackFormatter, Expression, FormatResult, LocalNodeId,
    expression_has_leading_prefix_comment, format_with, group, hard_line_break, indent,
    soft_line_break_or_space, space,
};
use crate::operator::{
    format_binary_operand_with_grouping_parentheses, is_object_like_type_expression,
    is_type_context, should_hug_nullable_union_type, should_hug_static_argument_union_type,
    type_binary_operand_needs_grouping_parentheses, union_has_leading_pipe_token,
};
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Format a binary expression with all operator-specific layout policies.
pub(in crate::format::operator) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_BINARY);
    let in_type_context = is_type_context(f.context(), node_id);
    let is_destack = f.context().options.language_type.is_destack();
    let is_type_intersection = in_type_context && *operator == BinaryOperator::ElementwiseAnd;
    let is_type_union = in_type_context && *operator == BinaryOperator::ElementwiseOr;

    // specialized logical and coalesce layout paths
    if logical::try_format_trailing_coalesce(f, node_id, left, *operator, right)?
        || logical::try_format_logical_right_prefix_block_comment(
            f, node_id, left, *operator, right,
        )?
        || logical::try_format_logical_right_prefix_line_comment(f, left, *operator, right)?
        || logical::try_format_mixed_logical_precedence(f, left, *operator, right)?
        || logical::try_format_logical_parenthesized_cases(f, node_id, left, *operator, right)?
    {
        return Ok(());
    }

    // flatten operands
    let operands = if is_type_union || is_type_intersection {
        flatten_type_binary_expression(f.context(), node_id, *operator)
    } else {
        flatten_binary_expression(f.context().tree, node_id, *operator)
    };
    let should_force_type_binary_expansion = is_type_intersection
        && type_binary_operands_are_structurally_complex(f.context(), &operands);

    // clean binary short-circuit
    let can_use_clean_binary_short_circuit = !is_type_union
        && !is_type_intersection
        && !f.context().has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !f.context().has_annotation(operand.expression))
        && operands
            .iter()
            .all(|operand| !expression_has_leading_prefix_comment(f.context(), operand.expression));
    if can_use_clean_binary_short_circuit {
        f.context()
            .increment_counter("profile.binary.clean.short_circuit", 1);
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let Some(first_operand) = operands.first() else {
                    return Ok(());
                };
                format_binary_operand_with_grouping_parentheses(
                    f,
                    *operator,
                    first_operand.expression,
                )?;

                for operand in operands.iter().skip(1) {
                    let Some(op) = operand.operator else {
                        continue;
                    };
                    write!(
                        f,
                        [
                            space(),
                            op,
                            indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )
                            }))
                        ]
                    )?;
                }

                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // leading pipe unions
    if is_type_union
        && logical::should_use_leading_pipe_union_style(f.context(), node_id, &operands)
    {
        logical::format_leading_pipe_union(f, node_id, &operands)?;
        return Ok(());
    }

    // nullable union hugging
    if is_type_union && should_hug_nullable_union_type(f.context(), &operands) {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                for operand in &operands {
                    if let Some(op) = operand.operator {
                        let has_postfix =
                            prev_expression.is_some_and(|e| f.context().has_postfix_annotation(e));
                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        write!(f, [op, space(), operand.expression])?;
                    } else {
                        write!(f, [operand.expression])?;
                    }
                    prev_expression = Some(operand.expression);
                }
                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // static argument union hugging
    if is_type_union && should_hug_static_argument_union_type(f.context(), node_id, &operands) {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                for operand in &operands {
                    if let Some(op) = operand.operator {
                        let has_postfix = prev_expression.is_some_and(|expression_id| {
                            f.context().has_postfix_annotation(expression_id)
                        });
                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        write!(f, [op, space(), operand.expression])?;
                    } else {
                        write!(f, [operand.expression])?;
                    }
                    prev_expression = Some(operand.expression);
                }
                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // destack intersection trailing operators
    if is_type_intersection && is_destack {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                let mut prev_object_like = false;
                let mut prev_has_annotation = false;

                for operand in &operands {
                    if let Some(op) = operand.operator {
                        let has_postfix =
                            prev_expression.is_some_and(|e| f.context().has_postfix_annotation(e));
                        let is_object_like =
                            is_object_like_type_expression(f.context(), operand.expression);
                        let current_has_annotation = f.context().has_annotation(operand.expression);
                        let allow_break = !(prev_object_like || is_object_like)
                            || prev_has_annotation
                            || current_has_annotation;

                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        write!(f, [op])?;

                        if allow_break {
                            write!(
                                f,
                                [indent(&format_with(
                                    |f: &mut DestackFormatter<'ast, '_>| {
                                        write!(f, [soft_line_break_or_space(), operand.expression])
                                    }
                                ))]
                            )?;
                        } else if is_object_like {
                            write!(
                                f,
                                [
                                    space(),
                                    indent(&format_with(|f| write!(f, [operand.expression])))
                                ]
                            )?;
                        } else {
                            write!(f, [space(), operand.expression])?;
                        }

                        prev_object_like = is_object_like;
                        prev_has_annotation = current_has_annotation;
                    } else {
                        write!(f, [operand.expression])?;
                        prev_object_like =
                            is_object_like_type_expression(f.context(), operand.expression);
                        prev_has_annotation = f.context().has_annotation(operand.expression);
                    }

                    prev_expression = Some(operand.expression);
                }

                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // clean type binary short-circuit
    let can_use_clean_type_binary_short_circuit = (is_type_union || is_type_intersection)
        && !(is_type_union && union_has_leading_pipe_token(f.context(), node_id))
        && !f.context().has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !f.context().has_annotation(operand.expression));
    if can_use_clean_type_binary_short_circuit {
        f.context()
            .increment_counter("profile.binary.type_clean.short_circuit", 1);
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let Some(first_operand) = operands.first() else {
                    return Ok(());
                };
                format_binary_operand_with_grouping_parentheses(
                    f,
                    *operator,
                    first_operand.expression,
                )?;

                for operand in operands.iter().skip(1) {
                    let Some(op) = operand.operator else {
                        continue;
                    };
                    write!(
                        f,
                        [
                            space(),
                            op,
                            indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )
                            }))
                        ]
                    )?;
                }

                Ok(())
            }))
            .should_expand(should_force_type_binary_expansion)]
        )?;
        return Ok(());
    }

    // default flattened binary formatting
    // default flattened binary formatting path
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut prev_expression: Option<LocalNodeId<Expression>> = None;
            // emit each flattened operand with operator-aware spacing
            for operand in &operands {
                // non-head operands write their leading operator
                if let Some(op) = operand.operator {
                    let has_postfix =
                        prev_expression.is_some_and(|e| f.context().has_postfix_annotation(e));
                    let has_existing_operator_break =
                        prev_expression.is_some_and(|previous_expression| {
                            has_newline_between_expressions(
                                f.context(),
                                previous_expression,
                                operand.expression,
                            )
                        });
                    let preserve_existing_operator_break = if is_type_union || is_type_intersection
                    {
                        false
                    } else {
                        preserve_existing_operator_break(op, has_existing_operator_break)
                    };
                    let previous_has_prefix_annotation =
                        prev_expression.is_some_and(|expression_id| {
                            expression_has_leading_prefix_comment(f.context(), expression_id)
                        });
                    let previous_has_line_prefix_slash_comment =
                        prev_expression.is_some_and(|expression_id| {
                            expression_has_line_prefix_slash_comment(f.context(), expression_id)
                        });
                    let previous_is_parenthesized_multiline =
                        prev_expression.is_some_and(|expression_id| {
                            matches!(
                                f.context().tree.get(expression_id),
                                Expression::Parenthesized { .. }
                            ) && f.context().node_has_newline(expression_id)
                        });
                    let previous_needs_grouping_parentheses_multiline = prev_expression
                        .is_some_and(|expression_id| {
                            type_binary_operand_needs_grouping_parentheses(
                                f.context(),
                                *operator,
                                expression_id,
                            ) && f.context().node_has_newline(expression_id)
                        });
                    let previous_is_parenthesized_or_grouped_multiline =
                        previous_is_parenthesized_multiline
                            || previous_needs_grouping_parentheses_multiline;
                    let previous_has_line_postfix_slash_comment =
                        prev_expression.is_some_and(|expression_id| {
                            expression_has_line_postfix_slash_comment(f.context(), expression_id)
                        });
                    let previous_requires_type_grouping_break =
                        (is_type_union || is_type_intersection) && has_postfix;
                    if op == BinaryOperator::ElementwiseAnd
                        && previous_is_parenthesized_or_grouped_multiline
                    {
                        write!(
                            f,
                            [
                                space(),
                                op,
                                indent(&format_args![
                                    hard_line_break(),
                                    format_with(|f| {
                                        format_binary_operand_with_grouping_parentheses(
                                            f,
                                            *operator,
                                            operand.expression,
                                        )
                                    })
                                ])
                            ]
                        )?;
                        prev_expression = Some(operand.expression);
                        continue;
                    }
                    let current_has_line_prefix_slash_comment =
                        expression_has_line_prefix_slash_comment(f.context(), operand.expression);
                    if (is_type_union || is_type_intersection)
                        && current_has_line_prefix_slash_comment
                    {
                        if !has_postfix {
                            write!(f, [space()])?;
                        } else if previous_requires_type_grouping_break
                            || previous_has_line_postfix_slash_comment
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(
                            f,
                            [
                                op,
                                space(),
                                indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    format_binary_operand_with_grouping_parentheses(
                                        f,
                                        *operator,
                                        operand.expression,
                                    )
                                }))
                            ]
                        )?;
                        prev_expression = Some(operand.expression);
                        continue;
                    }
                    let operand_prefers_trailing_operator =
                        operand_prefers_trailing_logical_operator(
                            f.context(),
                            op,
                            operand.expression,
                        );
                    if operand_prefers_trailing_operator {
                        let allow_space_after_line_comment =
                            prev_expression.is_some_and(|expression_id| {
                                logical_left_has_line_postfix_slash_comment(
                                    f.context(),
                                    op,
                                    expression_id,
                                )
                            });
                        if !has_postfix || allow_space_after_line_comment {
                            write!(f, [space()])?;
                        } else if previous_requires_type_grouping_break
                            || previous_has_line_postfix_slash_comment
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(
                            f,
                            [
                                op,
                                indent(&format_args![
                                    hard_line_break(),
                                    format_with(|f| {
                                        format_binary_operand_with_grouping_parentheses(
                                            f,
                                            *operator,
                                            operand.expression,
                                        )
                                    })
                                ])
                            ]
                        )?;
                    } else if (is_type_union || is_type_intersection)
                        && previous_has_line_prefix_slash_comment
                    {
                        if !has_postfix {
                            write!(f, [space()])?;
                        } else if previous_requires_type_grouping_break
                            || previous_has_line_postfix_slash_comment
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(
                            f,
                            [
                                op,
                                indent(&format_args![
                                    hard_line_break(),
                                    format_with(|f| {
                                        format_binary_operand_with_grouping_parentheses(
                                            f,
                                            *operator,
                                            operand.expression,
                                        )
                                    })
                                ])
                            ]
                        )?;
                    } else if previous_has_prefix_annotation {
                        let allow_space_after_line_comment =
                            prev_expression.is_some_and(|expression_id| {
                                logical_left_has_line_postfix_slash_comment(
                                    f.context(),
                                    op,
                                    expression_id,
                                )
                            });
                        if !has_postfix || allow_space_after_line_comment {
                            write!(f, [space()])?;
                        } else if previous_requires_type_grouping_break
                            || previous_has_line_postfix_slash_comment
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(f, [op, space()])?;
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            *operator,
                            operand.expression,
                        )?;
                    } else {
                        write!(
                            f,
                            [indent(&format_with(
                                |f: &mut DestackFormatter<'ast, '_>| {
                                    let allow_space_after_line_comment = prev_expression
                                        .is_some_and(|expression_id| {
                                            logical_left_has_line_postfix_slash_comment(
                                                f.context(),
                                                op,
                                                expression_id,
                                            )
                                        });
                                    if !has_postfix {
                                        if preserve_existing_operator_break {
                                            write!(f, [hard_line_break()])?;
                                        } else if previous_is_parenthesized_multiline
                                            && is_logical_binary_operator(op)
                                        {
                                            write!(f, [space()])?;
                                        } else {
                                            write!(f, [soft_line_break_or_space()])?;
                                        }
                                    } else if allow_space_after_line_comment {
                                        write!(f, [space()])?;
                                    } else if previous_requires_type_grouping_break
                                        || previous_has_line_postfix_slash_comment
                                    {
                                        write!(f, [hard_line_break()])?;
                                    }
                                    write!(f, [op, space()])?;
                                    format_binary_operand_with_grouping_parentheses(
                                        f,
                                        *operator,
                                        operand.expression,
                                    )
                                }
                            ))]
                        )?;
                    }
                // head operand keeps existing grouping rules
                } else {
                    let first_operand_has_prefix_annotation =
                        f.context().has_prefix_annotation(operand.expression);
                    let should_indent_first_operand = first_operand_has_prefix_annotation
                        && (is_type_union || is_type_intersection);
                    if should_indent_first_operand {
                        write!(
                            f,
                            [indent(&format_args![format_with(|f| {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )
                            })])]
                        )?;
                    } else {
                        format_binary_operand_with_grouping_parentheses(
                            f,
                            *operator,
                            operand.expression,
                        )?;
                    }
                }
                prev_expression = Some(operand.expression);
            }
            Ok(())
        }))
        .should_expand(should_force_type_binary_expansion)]
    )?;

    Ok(())
}
