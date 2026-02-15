use super::super::super::timing::tags;
use super::super::*;
use super::common::expression_is_trivial_inline_without_annotations;
use destack_fir::{format_args, write};

// binary inline width constants
const BINARY_OPERATOR_PADDING_WIDTH: usize = 3;

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

/// Return whether one binary operator is logical.
#[inline]
fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether an expression ends with a `//` postfix annotation.
fn expression_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.get_annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.tree.get::<Annotation>(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };

        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(*node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether mixed logical precedence should parenthesize the right expression.
#[inline]
fn is_mixed_logical_precedence_pair(
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
fn should_preserve_mixed_logical_grouping_for_comments(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right_span = context.get_span(right);
    span_has_comment(context, right_span) || has_comment_between_expressions(context, left, right)
}

/// Return whether a source operator break should be preserved.
#[inline]
fn preserve_source_operator_break(
    operator: BinaryOperator,
    has_source_operator_break: bool,
) -> bool {
    !is_logical_binary_operator(operator) && has_source_operator_break
}

/// Return whether a logical operand prefers trailing-operator layout.
#[inline]
fn operand_prefers_trailing_logical_operator(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator) && context.has_prefix_annotation(operand_expression)
}

/// Return whether a logical binary left operand ends with a line postfix slash comment.
fn logical_left_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    left: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator) && expression_has_line_postfix_slash_comment(context, left)
}

/// Write one separating space after the left operand when no postfix trivia exists.
fn write_space_after_binary_left_if_needed<'ast>(
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

/// Return whether type binary operands contain any slash-style comments.
fn type_binary_operands_have_slash_comments(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    operands.iter().any(|operand| {
        let Some(annotation_ids) = context.get_annotations(operand.expression) else {
            return false;
        };

        annotation_ids.into_iter().any(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(annotation_id);
            let Annotation::Comment { node, .. } = annotation else {
                return false;
            };
            let comment = context.tree.get::<destack_ast::Comment>(*node);
            comment.style == destack_ast::CommentStyle::Slash
        })
    })
}

/// Format type binary operands as a flat sequence with inline separators.
fn format_flat_type_binary_operands<'ast>(
    operator: BinaryOperator,
    operands: &BinaryOperands,
) -> impl Format<DestackFormatContext<'ast>> {
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
        for (index, operand) in operands.iter().enumerate() {
            if index > 0 {
                let previous_has_postfix = prev_expression
                    .is_some_and(|expression_id| f.context().has_postfix_annotation(expression_id));
                if previous_has_postfix {
                    write!(f, [operator, space()])?;
                } else {
                    write!(f, [space(), operator, space()])?;
                }
            }
            write!(f, [operand.expression])?;
            prev_expression = Some(operand.expression);
        }
        Ok(())
    })
}

/// Try formatting `??` using trailing-operator layout.
fn try_format_trailing_coalesce<'ast>(
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

/// Try formatting logical operators with a trailing source line comment between operands.
fn try_format_logical_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }

    let Some(line_comment) = line_comment_between_expressions(f.context(), left, right) else {
        return Ok(false);
    };

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
            format_with(|f| write_space_after_binary_left_if_needed(f, left, operator)),
            operator,
            space(),
            text(line_comment.as_str()),
            indent(&format_args![hard_line_break(), right_without_prefix])
        ])]
    )?;

    Ok(true)
}

/// Try formatting mixed logical precedence pairs with explicit right parentheses.
fn try_format_mixed_logical_precedence<'ast>(
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
fn try_format_logical_parenthesized_cases<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !is_logical_binary_operator(operator) {
        return Ok(false);
    }

    let left_span = f.context().get_span(left);
    let left_source = f.context().get_span_str(left_span);
    let left_has_multiline_parenthesized_tail =
        f.context().has_newline(left_span) && left_source.trim_end().ends_with(')');
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

    // keep tree rhs inline when within assignment-like width budget
    let is_parenthesized_tree = matches!(
        right_expression,
        Expression::Parenthesized { expression }
            if matches!(f.context().tree.get(*expression), Expression::TreeExpression { .. })
    );
    if !is_parenthesized_tree {
        return Ok(false);
    }

    let line_width = usize::from(f.context().options.line_width);
    let remaining_width =
        assignment_like_remaining_width(f.context(), node_id).unwrap_or(line_width);
    let left_len = expression_source_len(f.context(), left);
    let operator_len = binary_operator_len(&operator);
    let inline_len = left_len
        .saturating_add(operator_len)
        .saturating_add(BINARY_OPERATOR_PADDING_WIDTH);
    if inline_len > remaining_width {
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
fn should_use_leading_pipe_union_style(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
) -> bool {
    if union_source_has_leading_pipe(context, node_id) {
        return true;
    }

    if let Some(first_operand) = operands.first() {
        let first_span = context.get_span(first_operand.expression);
        let first_source = context.get_span_str(first_span);
        if first_source.trim_start().starts_with('|')
            || previous_non_whitespace_before_span(context, first_span) == Some('|')
        {
            return true;
        }
    }

    if !context.node_has_newline(node_id) {
        return false;
    }

    operands.iter().any(|operand| {
        let Some(annotation_ids) = context.get_annotations(operand.expression) else {
            return false;
        };

        annotation_ids.into_iter().any(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(annotation_id);
            let Annotation::Comment { node, position } = annotation else {
                return false;
            };
            if !matches!(
                position,
                AnnotationPosition::BlockPrefix
                    | AnnotationPosition::LinePrefix
                    | AnnotationPosition::BlockPostfix
                    | AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
            ) {
                return false;
            }

            let comment = context.tree.get::<destack_ast::Comment>(*node);
            comment.style == destack_ast::CommentStyle::Slash
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
    LeadingPipeUnionLayoutPolicy {
        should_indent_operands: !context.has_prefix_annotation(node_id)
            && !has_block_prefix_ancestor,
        prefer_space_before_first_pipe: has_block_prefix_ancestor,
    }
}

/// Write the first operand of a leading-pipe union.
fn write_first_leading_pipe_union_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    policy: LeadingPipeUnionLayoutPolicy,
    expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if policy.should_indent_operands {
        write!(
            f,
            [indent(&format_with(|f| {
                if policy.prefer_space_before_first_pipe {
                    write!(f, [token("|"), space(), expression])
                } else {
                    write!(
                        f,
                        [soft_line_break_or_space(), token("|"), space(), expression]
                    )
                }
            }))]
        )?;
        return Ok(());
    }

    if policy.prefer_space_before_first_pipe {
        write!(f, [token("|"), space(), expression])
    } else {
        write!(
            f,
            [soft_line_break_or_space(), token("|"), space(), expression]
        )
    }
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
                write!(f, [token("|"), space(), operand_expression])
            }))]
        )?;
        return Ok(());
    }

    if has_line_postfix_slash_comment || has_postfix {
        write!(f, [hard_line_break()])?;
    } else {
        write!(f, [soft_line_break_or_space()])?;
    }
    write!(f, [token("|"), space(), operand_expression])
}

/// Format leading-pipe union operands with shared annotation and comment policy.
fn format_leading_pipe_union<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
) -> FormatResult<()> {
    let policy = leading_pipe_union_layout_policy(f.context(), node_id);

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
    )
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

    // specialized logical and coalesce layout paths
    if try_format_trailing_coalesce(f, node_id, left, *operator, right)?
        || try_format_logical_line_comment(f, left, *operator, right)?
        || try_format_mixed_logical_precedence(f, left, *operator, right)?
        || try_format_logical_parenthesized_cases(f, node_id, left, *operator, right)?
    {
        return Ok(());
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
            line_comment_between_expressions(
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
    if is_type_union && should_use_leading_pipe_union_style(f.context(), node_id, &operands) {
        format_leading_pipe_union(f, node_id, &operands)?;
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

    // keep type unions and intersections flat unless slash-line comments require multiline layout
    if (is_type_union || is_type_intersection)
        && !is_destack
        && !type_binary_operands_have_slash_comments(f.context(), &operands)
        && operands
            .iter()
            .all(|operand| !f.context().node_has_newline(operand.expression))
    {
        write!(f, [format_flat_type_binary_operands(*operator, &operands)])?;
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
    // default flattened binary formatting path
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
            // emit each flattened operand with operator-aware spacing
            for operand in &operands {
                // non-head operands write their leading operator
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
                    let preserve_source_operator_break =
                        preserve_source_operator_break(op, has_source_operator_break);
                    let previous_has_prefix_annotation =
                        prev_expression.is_some_and(|expression_id| {
                            expression_has_leading_prefix_comment(f.context(), expression_id)
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
                                        if preserve_source_operator_break {
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

/// Write a cast or satisfies operator and right operand.
fn write_type_binary_operator_and_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [operator, space()])?;
    write!(f, [right])
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
                write_type_binary_operator_and_right(f, operator, right)
            }))]
        )?;
    } else {
        let keep_left_and_operator_on_same_line = matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        );

        write!(
            f,
            [group(&format_args![
                formatted_left,
                indent(&format_with(|f| {
                    if !has_postfix {
                        if left_has_leading_prefix_comment || keep_left_and_operator_on_same_line {
                            write!(f, [space()])?;
                        } else {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                    }
                    write_type_binary_operator_and_right(f, operator, right)
                }))
            ])]
        )?;
    }

    Ok(())
}
