use super::common::expression_is_trivial_inline_without_annotations;
use super::*;
use crate::analysis::scan::{
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
};
use destack_ast::{Comment, CommentStyle};
use destack_fir::{format_args, write};

// assignment inline width constants
const ASSIGNMENT_OPERATOR_PADDING_WIDTH: usize = 2;

// assignment shape thresholds
const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;
const EXPANDED_OBJECT_TARGET_PROPERTY_THRESHOLD: usize = 2;
const SHORT_OBJECT_PROPERTY_MAX: usize = 3;

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

/// Return whether one span contains at least one assignment operator token.
fn span_contains_assignment_operator_token(context: &DestackFormatContext<'_>, span: Span) -> bool {
    for token in context.tokens {
        if token.span.end <= span.start {
            continue;
        }
        if token.span.start >= span.end {
            break;
        }
        if is_assignment_operator_token(token.token.ty) {
            return true;
        }
    }

    false
}

/// Return whether one expression has a line-prefix slash comment on an assignment seam.
fn expression_has_assignment_seam_line_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().any(|annotation_id| {
                let Annotation::Comment { node, position } = context.annotation(*annotation_id)
                else {
                    return false;
                };
                if position != AnnotationPosition::LinePrefix {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                if comment.style != CommentStyle::Slash {
                    return false;
                }

                previous_non_whitespace_token_before_annotation(context, *annotation_id)
                    .is_some_and(|token| is_assignment_operator_token(token.token.ty))
            })
        })
        .unwrap_or(false)
}

/// Return whether one assignment seam has a slash line comment between left and right.
fn assignment_seam_has_line_comment_between(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left);
    let right_span = context.span(right);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_span = Span::new(left_span.file, left_span.end, right_span.start);
    context
        .comment_tokens()
        .iter()
        .copied()
        .any(|comment_token| {
            if !between_span.intersects(comment_token.span) {
                return false;
            }

            if !matches!(
                comment_token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                return false;
            }

            previous_non_whitespace_token_before_span(context, comment_token.span)
                .is_some_and(|token| is_assignment_operator_token(token.token.ty))
        })
}

/// Format an assignment expression with shared rhs break policy.
pub(super) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // fast path: trivia free, short, simple assignments stay inline
    let line_width = usize::from(f.context().options.line_width);
    let inner_right_id = transparent_inner_expression(f.context(), right);
    let inner_right_expr = f.context().tree.get(inner_right_id);
    let right_is_simple_expression = matches!(
        inner_right_expr,
        Expression::ScalarLiteral(_)
            | Expression::Path { .. }
            | Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
    );
    let has_assignment_parent =
        f.context()
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }
                matches!(
                    f.context()
                        .tree
                        .get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { .. }
                )
            });
    let left_has_annotation = f.context().has_annotation(left);
    let right_has_annotation = f.context().has_annotation(right);
    let node_has_annotation = f.context().has_annotation(node_id);
    if right_is_simple_expression
        && !has_assignment_parent
        && !left_has_annotation
        && !right_has_annotation
        && !node_has_annotation
    {
        let left_source_len = expression_source_len(f.context(), left);
        let operator_len = assign_operator_len(operator);
        let right_source_len = expression_source_len(f.context(), inner_right_id);
        let inline_len = left_source_len
            .saturating_add(operator_len)
            .saturating_add(right_source_len)
            .saturating_add(ASSIGNMENT_OPERATOR_PADDING_WIDTH);
        if inline_len <= line_width {
            f.context()
                .increment_counter("profile.assign.simple_inline.fast_path", 1);
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

    let has_postfix = f.context().has_postfix_annotation(left);

    // binaries, chains, and nested lambda tails handle their own breaking
    let right_is_binary = matches!(inner_right_expr, Expression::Binary { .. });
    let right_is_sequence = matches!(inner_right_expr, Expression::SequenceExpression { .. });
    let right_is_assign = matches!(inner_right_expr, Expression::Assign { .. });
    let right_is_chain_root = is_chain_root(f.context().tree, inner_right_id);
    let right_is_chain =
        is_expression_chain(f.context().tree, inner_right_id) || right_is_chain_root;
    let right_is_chain_tail_lambda =
        is_assignment_chain_tail_lambda(f.context(), node_id, inner_right_id);
    let right_is_lambda = is_lambda_expression(f.context(), inner_right_id);
    let right_handles_its_own_breaking = right_is_binary
        || right_is_sequence
        || right_is_assign
        || right_is_chain
        || right_is_chain_tail_lambda
        || right_is_lambda;
    let right_has_prefix_annotation = f.context().has_prefix_annotation(right);
    let right_has_assignment_seam_prefix_line_comment =
        expression_has_assignment_seam_line_prefix_comment(f.context(), right)
            || assignment_seam_has_line_comment_between(f.context(), left, right);
    let right_has_prefix_annotation_that_forces_operator_break =
        right_has_prefix_annotation && !right_has_assignment_seam_prefix_line_comment;
    let left_has_newline = f.context().node_has_newline(left);
    let right_has_newline = f.context().node_has_newline(right);
    let right_has_between_comment = has_comment_between_expressions(f.context(), left, right);

    // string literals are atomic: never break at `=`
    let is_string_literal = matches!(
        inner_right_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );

    // estimate remaining inline width for rhs
    let left_source_len = source_min_inline_char_len(f.context().span_str(f.context().span(left)));
    let operator_len = assign_operator_len(operator);
    let inline_overhead = left_source_len
        .saturating_add(operator_len)
        .saturating_add(ASSIGNMENT_OPERATOR_PADDING_WIDTH);
    let remaining_width = line_width.saturating_sub(inline_overhead);
    let right_source_len = expression_source_len(f.context(), inner_right_id);
    let right_annotation_len = expression_prefix_annotation_source_len(f.context(), right);
    let right_source_len = right_source_len.saturating_add(right_annotation_len);
    let right_is_long = right_source_len > remaining_width;
    let right_span = f.context().span(inner_right_id);
    let right_compact_source_len = source_min_inline_char_len(f.context().span_str(right_span))
        .saturating_add(right_annotation_len);
    let right_is_compact_long = right_compact_source_len > remaining_width;

    // prefer breaking after `=` for long binary rhs values
    let right_is_long_binary = if right_is_binary {
        let binary_operand_count = match inner_right_expr {
            Expression::Binary { operator, .. } => {
                flattened_binary_operand_count(f.context().tree, inner_right_id, *operator)
            }
            _ => 0,
        };
        right_source_len > remaining_width
            && binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
            && binary_rhs_prefers_break_after_operator(right_source_len, line_width)
    } else {
        false
    };

    if is_string_literal {
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

    // format the space before the operator, respecting postfix comments
    let space_before_operator = format_with(|f| {
        if !has_postfix {
            write!(f, [space()])?;
        }
        Ok(())
    });

    // keep assignment seam slash comments inline with the operator
    if right_has_assignment_seam_prefix_line_comment {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                indent(&right)
            ])]
        )?;
        return Ok(());
    }

    // break long binary rhs values after the operator
    if right_is_long_binary {
        let format_break_after_operator = format_with(|f| {
            // avoid double indentation when the rhs breaks internally
            let dedented_right = dedent(&right);
            write!(
                f,
                [
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), dedented_right])
                ]
            )
        });

        let format_inline = format_with(|f| {
            group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])
            .format(f)
        });

        if right_is_long {
            format_break_after_operator.format(f)?;
        } else {
            format_inline.format(f)?;
        }
        return Ok(());
    }

    let right_is_collection_or_call_like = matches!(
        inner_right_expr,
        Expression::ObjectExpression { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::Call { .. }
            | Expression::New { .. }
            | Expression::Instantiation { .. }
    );
    let right_is_anonymous_class_declaration = matches!(
        inner_right_expr,
        Expression::Declaration(declaration_id)
            if matches!(
                f.context().tree.get(*declaration_id),
                Declaration::Class { descriptor, .. } if descriptor.name.is_none()
            )
    );
    if (right_is_collection_or_call_like || right_is_anonymous_class_declaration)
        && !right_has_prefix_annotation
        && !right_has_between_comment
    {
        write!(
            f,
            [group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])]
        )?;
        return Ok(());
    }

    if right_handles_its_own_breaking {
        let right_prefers_operator_break = if right_is_assign {
            // nested assignment chains: break by stable shape signals only
            !right_is_lambda
                && (right_is_compact_long
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment)
        } else if right_is_lambda {
            right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || right_is_compact_long
        } else if right_is_chain {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || right_has_prefix_annotation_that_forces_operator_break
                    || (right_is_long
                        && (remaining_width == 0 || left_has_newline || !right_has_newline))
                    || right_has_between_comment)
        }
        // binary rhs values should not depend on source-only break signals, to keep idempotence stable
        else if right_is_binary {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment)
                && (right_is_compact_long
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_has_between_comment)
        } else {
            !right_is_lambda
                && (right_is_chain_tail_lambda
                    || right_has_prefix_annotation_that_forces_operator_break
                    || right_is_compact_long
                    || right_has_between_comment)
        };

        let format_break_after_operator = format_with(|f| {
            if right_has_prefix_annotation_that_forces_operator_break
                || right_has_between_comment
                || right_is_sequence
            {
                write!(
                    f,
                    [
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![hard_line_break(), right])
                    ]
                )
            } else {
                let dedented_right = dedent(&right);
                write!(
                    f,
                    [
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![hard_line_break(), dedented_right])
                    ]
                )
            }
        });

        let format_inline = format_with(|f| {
            group(&format_args![
                left,
                space_before_operator,
                operator,
                space(),
                right
            ])
            .format(f)
        });

        if right_prefers_operator_break {
            format_break_after_operator.format(f)?;
        } else {
            format_inline.format(f)?;
        }
    } else {
        let left_inner_id = transparent_inner_expression(f.context(), left);
        let left_is_assignment_chain = matches!(
            f.context().tree.get(left_inner_id),
            Expression::Assign { .. }
        );
        let left_source_has_assignment_operator =
            span_contains_assignment_operator_token(f.context(), f.context().span(left));
        let left_is_expanded_object_target = matches!(
            f.context().tree.get(left_inner_id),
            Expression::ObjectExpression { properties, .. }
                if properties.len() > EXPANDED_OBJECT_TARGET_PROPERTY_THRESHOLD
                    && is_assignment_left_target(f.context(), left_inner_id)
        );
        let right_is_short_object = matches!(
            inner_right_expr,
            Expression::ObjectExpression { properties, .. } if properties.len() <= SHORT_OBJECT_PROPERTY_MAX
        );
        let right_is_inline_atomic =
            expression_is_trivial_inline_without_annotations(f.context(), inner_right_id)
                && right_source_len <= remaining_width;
        let should_force_break_for_assignment_left_chain = (left_is_assignment_chain
            || left_source_has_assignment_operator)
            && right_is_compact_long
            && !right_is_short_object;
        let should_force_break_for_multiline_left =
            left_has_newline && !right_is_short_object && !right_is_inline_atomic;

        if should_force_break_for_assignment_left_chain || should_force_break_for_multiline_left {
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![hard_line_break(), dedent(&right)])
                ])]
            )?;
        } else if (left_has_newline || left_is_expanded_object_target)
            && (right_is_short_object || right_is_inline_atomic)
        {
            write!(f, [left, space_before_operator, operator, space(), right])?;
        } else {
            // other expressions get indented on break
            write!(
                f,
                [group(&format_args![
                    left,
                    space_before_operator,
                    operator,
                    indent(&format_args![soft_line_break_or_space(), right])
                ])]
            )?;
        }
    }

    Ok(())
}
