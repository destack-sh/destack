use super::super::super::timing::tags;
use super::super::*;
use super::shared::expression_is_trivial_inline_without_annotations;
use destack_fir::{format_args, write};

/// Return whether a leading type union has an expression ancestor with block-prefix comments.
fn leading_union_has_ancestor_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;
    while let Some((parent_id, parent_type)) = context.get_parent(current_id) {
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

/// Format a binary expression with all operator-specific layout policies.
pub(super) fn format_binary_expression<'ast>(
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

    // trailing coalesce
    if *operator == BinaryOperator::Coalesce
        && should_use_trailing_coalesce(f.context(), node_id, left)
    {
        let has_postfix = f.context().has_postfix_annotation(left);
        write!(
            f,
            [group(&format_args![
                left,
                indent(&format_with(|f| {
                    if !has_postfix {
                        write!(f, [space()])?;
                    }
                    write!(f, [operator, soft_line_break_or_space(), right])
                }))
            ])]
        )?;
        return Ok(());
    }

    // operator trailing line comment
    if matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    ) && let Some(line_comment) = line_comment_between_expressions(f.context(), left, right)
    {
        let has_postfix = f.context().has_postfix_annotation(left);
        let right_without_prefix = format_with(|f| {
            let right_directive = directive_for_node(f.context(), right);
            format_expression(f, right, f.context().tree.get(right), right_directive)?;
            if !matches!(
                right_directive,
                Some(FormatterDirective {
                    kind: FormatterDirectiveKind::IgnoreFormat,
                    position: FormatterDirectivePosition::Postfix { .. },
                })
            ) {
                write!(f, [f.context().any_infix_or_postfix_annotations(right)])?;
            }
            Ok(())
        });
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| {
                    if !has_postfix {
                        write!(f, [space()])?;
                    }
                    Ok(())
                }),
                operator,
                space(),
                text(line_comment.as_str()),
                indent(&format_args![hard_line_break(), right_without_prefix])
            ])]
        )?;
        return Ok(());
    }

    // mixed logical precedence
    if matches!(operator, BinaryOperator::Or | BinaryOperator::Coalesce)
        && let Expression::Binary {
            operator: right_operator,
            ..
        } = f.context().tree.get(right)
        && *right_operator != *operator
        && matches!(
            right_operator,
            BinaryOperator::And | BinaryOperator::Coalesce
        )
    {
        let has_postfix = f.context().has_postfix_annotation(left);
        write!(
            f,
            [group(&format_args![
                left,
                format_with(|f| {
                    if !has_postfix {
                        write!(f, [space()])?;
                    }
                    Ok(())
                }),
                operator,
                space(),
                token("("),
                right,
                token(")")
            ])]
        )?;
        return Ok(());
    }

    // logical operands with parenthesized trees
    if matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    ) {
        let left_span = f.context().get_span(left);
        let left_source = f.context().get_span_str(left_span);
        let left_has_multiline_parenthesized_tail =
            f.context().has_newline(left_span) && left_source.trim_end().ends_with(')');
        let right_expression = f.context().tree.get(right);
        let right_is_inline_trivial =
            expression_is_trivial_inline_without_annotations(f.context(), right);
        let right_has_prefix = f.context().has_prefix_annotation(right);
        if left_has_multiline_parenthesized_tail
            && right_is_inline_trivial
            && !right_has_prefix
            && *operator == BinaryOperator::And
        {
            let has_postfix = f.context().has_postfix_annotation(left);
            write!(
                f,
                [group(&format_args![
                    left,
                    format_with(|f| {
                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        Ok(())
                    }),
                    operator,
                    indent(&format_args![hard_line_break(), right])
                ])]
            )?;
            return Ok(());
        }

        let left_prefers_trailing_operator = matches!(
            f.context().tree.get(left),
            Expression::Parenthesized { expression }
                if (f.context().has_prefix_annotation(left)
                    || f.context().has_prefix_annotation(*expression))
                    && f.context().has_newline(f.context().get_span(left))
        );
        if left_prefers_trailing_operator
            && expression_is_trivial_inline_without_annotations(f.context(), right)
        {
            let has_postfix = f.context().has_postfix_annotation(left);
            write!(
                f,
                [group(&format_args![
                    left,
                    format_with(|f| {
                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        Ok(())
                    }),
                    operator,
                    space(),
                    right
                ])]
            )?;
            return Ok(());
        }

        let is_parenthesized_tree = matches!(
            right_expression,
            Expression::Parenthesized { expression }
                if matches!(f.context().tree.get(*expression), Expression::TreeExpression { .. })
        );

        if is_parenthesized_tree {
            let line_width = usize::from(f.context().options.line_width);
            let remaining_width =
                assignment_like_remaining_width(f.context(), node_id).unwrap_or(line_width);
            let left_len = expression_source_len(f.context(), left);
            let operator_len = binary_operator_len(operator);
            let inline_len = left_len.saturating_add(operator_len).saturating_add(3);

            if inline_len <= remaining_width {
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
                return Ok(());
            }
        }
    }

    // flatten operands
    let operands = if is_type_union || is_type_intersection {
        flatten_type_binary_expression(f.context(), node_id, *operator)
    } else {
        flatten_binary_expression(f.context().tree, node_id, *operator)
    };

    // clean binary fast path
    let can_use_clean_binary_fast_path = !is_type_union
        && !is_type_intersection
        && !f.context().has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !f.context().has_annotation(operand.expression))
        && operands
            .iter()
            .all(|operand| !expression_has_leading_prefix_comment(f.context(), operand.expression))
        && operands.windows(2).all(|window| {
            let [left_operand, right_operand] = window else {
                return true;
            };
            !has_newline_between_expressions(
                f.context(),
                left_operand.expression,
                right_operand.expression,
            ) && line_comment_between_expressions(
                f.context(),
                left_operand.expression,
                right_operand.expression,
            )
            .is_none()
        });
    if can_use_clean_binary_fast_path {
        f.context()
            .increment_counter("profile.binary.clean.fast_path", 1);
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
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space(), op, space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )
                            }
                        ))]
                    )?;
                }

                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // leading pipe unions
    if is_type_union && union_source_has_leading_pipe(f.context(), node_id) {
        let has_block_prefix_ancestor =
            leading_union_has_ancestor_block_prefix_annotation(f.context(), node_id);
        let should_indent_leading_pipe_operands =
            !f.context().has_prefix_annotation(node_id) && !has_block_prefix_ancestor;
        let prefer_space_before_first_leading_pipe = has_block_prefix_ancestor;
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                for (index, operand) in operands.iter().enumerate() {
                    if index == 0 {
                        if should_indent_leading_pipe_operands {
                            write!(
                                f,
                                [indent(&format_with(|f| {
                                    if prefer_space_before_first_leading_pipe {
                                        write!(f, [token("|"), space(), operand.expression])
                                    } else {
                                        write!(
                                            f,
                                            [
                                                soft_line_break_or_space(),
                                                token("|"),
                                                space(),
                                                operand.expression
                                            ]
                                        )
                                    }
                                }))]
                            )?;
                        } else if prefer_space_before_first_leading_pipe {
                            write!(f, [token("|"), space(), operand.expression])?;
                        } else {
                            write!(
                                f,
                                [
                                    soft_line_break_or_space(),
                                    token("|"),
                                    space(),
                                    operand.expression
                                ]
                            )?;
                        }
                    } else {
                        let has_postfix = prev_expression.is_some_and(|expression_id| {
                            f.context().has_postfix_annotation(expression_id)
                        });
                        let between_line_comment = prev_expression.and_then(|expression_id| {
                            line_comment_between_expressions(
                                f.context(),
                                expression_id,
                                operand.expression,
                            )
                        });
                        let operand_without_prefix = format_with(|f| {
                            let operand_expression = normalize_type_binary_operand_expression(
                                f.context(),
                                operand.expression,
                                *operator,
                            );
                            let directive = directive_for_node(f.context(), operand_expression);
                            format_expression(
                                f,
                                operand_expression,
                                f.context().tree.get(operand_expression),
                                directive,
                            )?;
                            if !matches!(
                                directive,
                                Some(FormatterDirective {
                                    kind: FormatterDirectiveKind::IgnoreFormat,
                                    position: FormatterDirectivePosition::Postfix { .. },
                                })
                            ) {
                                write!(
                                    f,
                                    [f.context()
                                        .any_infix_or_postfix_annotations(operand_expression)]
                                )?;
                            }
                            Ok(())
                        });
                        if should_indent_leading_pipe_operands {
                            write!(
                                f,
                                [indent(&format_with(|f| {
                                    if !has_postfix {
                                        write!(f, [soft_line_break_or_space()])?;
                                    }
                                    if let Some(line_comment) = between_line_comment.as_ref() {
                                        write!(
                                            f,
                                            [
                                                text(line_comment.as_str()),
                                                hard_line_break(),
                                                token("|"),
                                                space(),
                                                operand_without_prefix
                                            ]
                                        )
                                    } else {
                                        write!(f, [token("|"), space(), operand_without_prefix])
                                    }
                                }))]
                            )?;
                        } else {
                            if !has_postfix {
                                write!(f, [soft_line_break_or_space()])?;
                            }
                            if let Some(line_comment) = between_line_comment.as_ref() {
                                write!(
                                    f,
                                    [
                                        text(line_comment.as_str()),
                                        hard_line_break(),
                                        token("|"),
                                        space(),
                                        operand_without_prefix
                                    ]
                                )?;
                            } else {
                                write!(f, [token("|"), space(), operand_without_prefix])?;
                            }
                        }
                    }
                    prev_expression = Some(operand.expression);
                }

                Ok(())
            }))
            .should_expand(true)]
        )?;
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

    // clean type binary fast path
    let can_use_clean_type_binary_fast_path = (is_type_union || is_type_intersection)
        && !(is_type_union && union_source_has_leading_pipe(f.context(), node_id))
        && !f.context().has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !f.context().has_annotation(operand.expression));
    if can_use_clean_type_binary_fast_path {
        f.context()
            .increment_counter("profile.binary.type_clean.fast_path", 1);
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
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [soft_line_break_or_space(), op, space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )
                            }
                        ))]
                    )?;
                }

                Ok(())
            }))]
        )?;
        return Ok(());
    }

    // default flattened binary formatting
    let binary_parent_is_parenthesized =
        f.context()
            .get_parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                parent_type == NodeType::Expression
                    && matches!(
                        f.context().tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Parenthesized { expression } if *expression == node_id
                    )
            });
    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut prev_expression: Option<LocalNodeId<Expression>> = None;
            for operand in &operands {
                if let Some(op) = operand.operator {
                    let has_postfix =
                        prev_expression.is_some_and(|e| f.context().has_postfix_annotation(e));
                    let has_source_operator_break =
                        prev_expression.is_some_and(|previous_expression| {
                            has_newline_between_expressions(
                                f.context(),
                                previous_expression,
                                operand.expression,
                            )
                        });
                    let previous_has_prefix_annotation =
                        prev_expression.is_some_and(|expression_id| {
                            expression_has_leading_prefix_comment(f.context(), expression_id)
                        });
                    let operand_prefers_trailing_operator =
                        matches!(
                            op,
                            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
                        ) && (f.context().has_prefix_annotation(operand.expression)
                            || f.context()
                                .has_newline(f.context().get_span(operand.expression)));
                    if operand_prefers_trailing_operator {
                        if !has_postfix {
                            write!(f, [space()])?;
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
                        if !has_postfix {
                            write!(f, [space()])?;
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
                                    if !has_postfix {
                                        if has_source_operator_break {
                                            write!(f, [hard_line_break()])?;
                                        } else {
                                            write!(f, [soft_line_break_or_space()])?;
                                        }
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
                } else {
                    let first_operand_has_prefix_annotation =
                        f.context().has_prefix_annotation(operand.expression);
                    let should_indent_first_operand = first_operand_has_prefix_annotation
                        && (binary_parent_is_parenthesized
                            || is_type_union
                            || is_type_intersection);
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
        }))]
    )?;

    Ok(())
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(super) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let mut formatted_left = left;
    if matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && let Expression::Parenthesized { expression } = f.context().tree.get(left)
        && parenthesized_should_drop(
            f.context(),
            left,
            *expression,
            ParenthesizedDropPolicy::TypeBinaryLeft { node_id },
        )
    {
        formatted_left = *expression;
    }

    let has_postfix = f.context().has_postfix_annotation(formatted_left);
    let left_has_leading_prefix_comment =
        expression_has_leading_prefix_comment(f.context(), formatted_left);
    let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
        || is_chain_root(f.context().tree, formatted_left);
    let line_width = usize::from(f.context().options.line_width);
    let is_parenthesized_new_callee = type_binary_is_parenthesized_new_callee(f.context(), node_id);
    let should_expand_chain_left = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && left_is_chain_expression
        && (expression_source_len(f.context(), node_id) > line_width
            || is_parenthesized_new_callee);

    if should_expand_chain_left {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(f, [formatted_left])
                    }))
                    .should_expand(true)]
                )?;
                if !has_postfix {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])
            }))]
        )?;
    } else {
        write!(
            f,
            [group(&format_args![
                formatted_left,
                indent(&format_with(|f| {
                    if !has_postfix {
                        if left_has_leading_prefix_comment {
                            write!(f, [space()])?;
                        } else {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                    }
                    write!(f, [operator, space(), right])
                }))
            ])]
        )?;
    }

    Ok(())
}
