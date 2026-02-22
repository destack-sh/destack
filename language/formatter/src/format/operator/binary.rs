use crate::Annotation;
use crate::format::analysis::scan::{
    first_non_trivia_token_in_span, last_non_trivia_token_in_span, next_non_whitespace_after_span,
};
use crate::format::analysis::timing::tags;
use crate::format::call::argument_satisfies_static_seam_comment_annotation_id;
use crate::format::chain::{
    BinaryOperands, flatten_binary_expression, flatten_type_binary_expression,
    has_newline_between_expressions, is_chain_root, is_expression_chain,
    should_use_trailing_coalesce,
};
use crate::format::expression::{
    Argument, BinaryOperator, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    LocalNodeId, ParenthesizedDropPolicy, TokenType, TypeBinaryOperator,
    expression_has_leading_prefix_comment,
    expression_has_non_doc_multiline_block_prefix_comment_annotation, format_with, group,
    hard_line_break, indent, parenthesized_should_drop, soft_line_break_or_space, space,
    span_has_comment, token, type_binary_is_parenthesized_new_callee,
    type_binary_is_parenthesized_statement_expression, type_binary_is_statement_expression,
};
use crate::format::operator::{
    AnnotationPosition, NodeType, expression_is_trivial_inline_without_annotations,
    format_binary_operand_with_grouping_parentheses,
    format_binary_operand_without_prefix_annotations_with_grouping_parentheses,
    has_comment_between_expressions, is_object_like_type_expression,
    is_static_type_argument_context, is_type_context, should_hug_nullable_union_type,
    should_hug_static_argument_union_type, type_binary_operand_needs_grouping_parentheses,
    union_has_leading_pipe_token,
};
use destack_ast::{Comment, CommentStyle};
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Select one binary render layout for one flattened operand set.
#[derive(Clone, Copy)]
pub(crate) enum BinaryLayout {
    /// Use the clean non-type short-circuit emitter.
    CleanShortCircuit,
    /// Use leading-pipe union rendering.
    LeadingPipeUnion,
    /// Use nullable union hugging.
    NullableUnionHug,
    /// Use static-argument union hugging.
    StaticArgumentUnionHug,
    /// Use destack intersection trailing operators.
    DestackIntersectionTrailing,
    /// Use the clean type-binary short-circuit emitter.
    CleanTypeShortCircuit,
    /// Use the default flattened emitter.
    DefaultFlattened,
}

/// Store binary render analysis outputs.
pub(crate) struct BinaryLayoutPlan {
    /// The selected layout.
    pub(crate) layout: BinaryLayout,
    /// The flattened operands.
    pub(crate) operands: BinaryOperands,
    /// Whether the final group should force expansion.
    pub(crate) should_force_type_binary_expansion: bool,
    /// Whether this binary is a type union.
    pub(crate) is_type_union: bool,
    /// Whether this binary is a type intersection.
    pub(crate) is_type_intersection: bool,
}

/// Analyze one binary expression into flattened operands and one layout decision.
pub(crate) fn analyze_binary_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operator: BinaryOperator,
) -> BinaryLayoutPlan {
    let in_type_context = is_type_context(context, node_id);
    let is_destack = context.options.language_type.is_destack();
    let is_type_intersection = in_type_context && operator == BinaryOperator::ElementwiseAnd;
    let is_type_union = in_type_context && operator == BinaryOperator::ElementwiseOr;

    let operands = if is_type_union || is_type_intersection {
        flatten_type_binary_expression(context, node_id, operator)
    } else {
        flatten_binary_expression(context.tree, node_id, operator)
    };
    let should_force_type_binary_expansion =
        is_type_intersection && type_binary_operands_are_structurally_complex(context, &operands);

    let can_use_clean_binary_short_circuit = !is_type_union
        && !is_type_intersection
        && !context.has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !context.has_annotation(operand.expression))
        && operands
            .iter()
            .all(|operand| !expression_has_leading_prefix_comment(context, operand.expression));
    if can_use_clean_binary_short_circuit {
        return BinaryLayoutPlan {
            layout: BinaryLayout::CleanShortCircuit,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    if is_type_union && should_use_leading_pipe_union_style(context, node_id, &operands) {
        return BinaryLayoutPlan {
            layout: BinaryLayout::LeadingPipeUnion,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    if is_type_union && should_hug_nullable_union_type(context, &operands) {
        return BinaryLayoutPlan {
            layout: BinaryLayout::NullableUnionHug,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    if is_type_union && should_hug_static_argument_union_type(context, node_id, &operands) {
        return BinaryLayoutPlan {
            layout: BinaryLayout::StaticArgumentUnionHug,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    if is_type_intersection && is_destack {
        return BinaryLayoutPlan {
            layout: BinaryLayout::DestackIntersectionTrailing,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    let can_use_clean_type_binary_short_circuit = (is_type_union || is_type_intersection)
        && !(is_type_union && union_has_leading_pipe_token(context, node_id))
        && !context.has_annotation(node_id)
        && operands
            .iter()
            .all(|operand| !context.has_annotation(operand.expression));
    if can_use_clean_type_binary_short_circuit {
        return BinaryLayoutPlan {
            layout: BinaryLayout::CleanTypeShortCircuit,
            operands,
            should_force_type_binary_expansion,
            is_type_union,
            is_type_intersection,
        };
    }

    BinaryLayoutPlan {
        layout: BinaryLayout::DefaultFlattened,
        operands,
        should_force_type_binary_expansion,
        is_type_union,
        is_type_intersection,
    }
}

/// Return whether type-binary operands are structurally complex enough to prefer multiline layout.
pub(crate) fn type_binary_operands_are_structurally_complex(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    if operands.len() > 3 {
        return true;
    }

    operands.iter().any(|operand| {
        context.node_has_newline(operand.expression)
            || !expression_is_trivial_inline_without_annotations(context, operand.expression)
    })
}

/// Return whether a leading type union has an expression ancestor with block-prefix comments.
pub(crate) fn leading_union_has_ancestor_block_prefix_annotation(
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
pub(crate) fn expression_parent_is_parenthesized(
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
pub(crate) fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
    )
}

/// Return whether an expression ends with a `//` postfix annotation.
pub(crate) fn expression_has_line_postfix_slash_comment(
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

        let comment = context.tree.get::<Comment>(node);
        comment.style == CommentStyle::Slash
    })
}

/// Return whether an expression starts with a `//` line-prefix annotation.
pub(crate) fn expression_has_line_prefix_slash_comment(
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

        let comment = context.tree.get::<Comment>(node);
        comment.style == CommentStyle::Slash
    })
}

/// Return whether an expression starts with an inline `/* ... */` prefix annotation.
pub(crate) fn expression_has_inline_block_prefix_star_comment(
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

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        !context.has_newline(annotation_span)
    })
}

/// Return whether root prefix comments should keep the first leading `|` inline.
pub(crate) fn leading_union_root_prefers_inline_first_pipe(
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
pub(crate) fn is_mixed_logical_precedence_pair(
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
pub(crate) fn should_preserve_mixed_logical_grouping_for_comments(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right_span = context.span(right);
    span_has_comment(context, right_span) || has_comment_between_expressions(context, left, right)
}

/// Return whether a source operator break should be preserved.
#[inline]
pub(crate) fn preserve_existing_operator_break(
    operator: BinaryOperator,
    has_existing_operator_break: bool,
) -> bool {
    (!is_logical_binary_operator(operator) || operator == BinaryOperator::Coalesce)
        && has_existing_operator_break
}

/// Return whether a logical operand prefers trailing-operator layout.
#[inline]
pub(crate) fn operand_prefers_trailing_logical_operator(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    operand_expression: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator)
        && expression_has_leading_prefix_comment(context, operand_expression)
}

/// Return whether a logical binary left operand ends with a line postfix slash comment.
pub(crate) fn logical_left_has_line_postfix_slash_comment(
    context: &DestackFormatContext<'_>,
    operator: BinaryOperator,
    left: LocalNodeId<Expression>,
) -> bool {
    is_logical_binary_operator(operator) && expression_has_line_postfix_slash_comment(context, left)
}

/// Write one separating space after the left operand when no postfix trivia exists.
pub(crate) fn write_space_after_binary_left_if_needed<'ast>(
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

/// Try formatting `??` using trailing-operator layout.
pub(crate) fn try_format_trailing_coalesce<'ast>(
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
pub(crate) fn try_format_logical_right_prefix_line_comment<'ast>(
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
pub(crate) fn try_format_logical_right_prefix_block_comment<'ast>(
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
pub(crate) fn try_format_mixed_logical_precedence<'ast>(
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
pub(crate) fn try_format_logical_parenthesized_cases<'ast>(
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
pub(crate) fn should_use_leading_pipe_union_style(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    operands: &BinaryOperands,
) -> bool {
    let is_static_type_argument = is_static_type_argument_context(context, node_id);
    let is_template_literal_interpolation =
        context.expression_is_in_template_literal_interpolation(node_id);
    let has_structural_complexity =
        type_binary_operands_are_structurally_complex(context, operands);
    let has_leading_pipe_token = union_has_leading_pipe_token(context, node_id);
    if is_template_literal_interpolation && !has_leading_pipe_token {
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

    if has_layout_forcing_comments {
        return true;
    }

    if is_static_type_argument && !has_leading_pipe_token {
        return false;
    }

    if has_leading_pipe_token {
        return has_many_union_operands
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

            let comment = context.tree.get::<Comment>(node);
            let is_last_operand = index + 1 == operands.len();
            if is_last_operand && comment.style == CommentStyle::Slash {
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
pub(crate) fn format_leading_pipe_union<'ast>(
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

/// Write a cast or satisfies operator and right operand.
fn write_type_binary_operator_and_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [operator, space()])?;
    write!(f, [right])
}

/// Return whether this cast expression should keep TypeScript angle assertion syntax.
fn cast_prefers_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    first_non_trivia_token_in_span(context, main_span)
        .is_some_and(|token| token.token.ty == TokenType::LessThan)
}

/// Return one satisfies seam line comment node from rhs ownership variants.
fn satisfies_seam_comment_node_id(
    context: &DestackFormatContext<'_>,
    right_expression_id: LocalNodeId<Expression>,
    static_arguments: &[LocalNodeId<Argument>],
) -> Option<LocalNodeId<Comment>> {
    if let Some(annotation_ids) = context.annotations(right_expression_id) {
        for annotation_id in annotation_ids {
            let Annotation::Comment {
                node,
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            } = context.annotation(annotation_id)
            else {
                continue;
            };

            let comment = context.tree.get::<Comment>(node);
            if comment.style != CommentStyle::Slash {
                continue;
            }

            return Some(node);
        }
    }

    static_arguments.first().and_then(|argument_id| {
        let annotation_id =
            argument_satisfies_static_seam_comment_annotation_id(context, *argument_id)?;
        let Annotation::Comment { node, .. } = context.annotation(annotation_id) else {
            return None;
        };
        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Slash {
            return None;
        }
        Some(node)
    })
}

/// Try to write one trivial object literal inline for satisfies seam comment layout.
fn try_write_inline_object_left_for_satisfies_seam_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(expression_id)
    else {
        return Ok(false);
    };
    if properties.len() != 1 {
        return Ok(false);
    }
    if f.context().has_annotation(expression_id) || f.context().node_has_newline(expression_id) {
        return Ok(false);
    }

    let property_id = properties[0];
    if f.context().has_annotation(property_id)
        || f.context().node_has_newline(property_id)
        || span_has_comment(f.context(), f.context().span(property_id))
    {
        return Ok(false);
    }

    write!(f, [token("{"), space(), property_id, space(), token("}")])?;
    Ok(true)
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(crate) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && cast_prefers_angle_assertion_syntax(f.context(), node_id);

    let mut formatted_left = left;
    if !cast_uses_angle_assertion
        && matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        )
        && let Expression::Parenthesized { expression } = f.context().tree.get(left)
        && parenthesized_should_drop(
            f.context(),
            left,
            *expression,
            ParenthesizedDropPolicy::TypeBinaryLeft { node_id },
        )
    {
        formatted_left = *expression;
    }

    // statement-level satisfies/cast over object literals should keep `({ ... })` lhs wrapping
    let left_needs_statement_object_parentheses =
        matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        ) && matches!(
            f.context().tree.get(formatted_left),
            Expression::ObjectExpression { .. }
        ) && (type_binary_is_statement_expression(f.context(), node_id)
            || type_binary_is_parenthesized_statement_expression(f.context(), node_id));

    let format_left = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        if left_needs_statement_object_parentheses {
            write!(f, [token("("), formatted_left, token(")")])
        } else {
            write!(f, [formatted_left])
        }
    };

    if cast_uses_angle_assertion {
        write!(f, [token("<"), right, token(">"), format_with(format_left)])?;
        return Ok(());
    }

    let has_postfix = f.context().has_postfix_annotation(formatted_left);
    let left_has_leading_prefix_comment =
        expression_has_leading_prefix_comment(f.context(), formatted_left);
    let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
        || is_chain_root(f.context().tree, formatted_left);
    let is_parenthesized_new_callee = type_binary_is_parenthesized_new_callee(f.context(), node_id);

    // satisfies separator seam comments before multi-argument static type lists:
    // `... satisfies // note\nRecord<A, B>` -> `... satisfies Record< // note\n    A,\n    B\n>`
    if *operator == TypeBinaryOperator::Satisfies
        && let Expression::Path {
            path,
            static_arguments: Some(static_arguments),
        } = f.context().tree.get(right)
        && static_arguments.len() > 1
        && path.segments.len() == 1
        && let Some(seam_comment_id) =
            satisfies_seam_comment_node_id(f.context(), right, static_arguments)
    {
        let wrote_inline_object_left =
            try_write_inline_object_left_for_satisfies_seam_comment(f, formatted_left)?;
        if !wrote_inline_object_left {
            write!(f, [group(&format_with(format_left))])?;
        }
        if !has_postfix {
            write!(f, [space()])?;
        }
        write!(
            f,
            [
                operator,
                space(),
                path.segments[0],
                token("<"),
                space(),
                seam_comment_id
            ]
        )?;
        write!(
            f,
            [indent(&format_with(|f| {
                write!(f, [hard_line_break()])?;
                for (index, argument_id) in static_arguments.iter().enumerate() {
                    write!(f, [*argument_id])?;
                    if index + 1 < static_arguments.len() {
                        write!(f, [token(","), hard_line_break()])?;
                    }
                }
                Ok(())
            }))]
        )?;
        write!(f, [hard_line_break(), token(">")])?;
        return Ok(());
    }

    let should_expand_chain_left = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && left_is_chain_expression
        && (f.context().node_has_newline(node_id)
            || f.context().node_has_newline(formatted_left)
            || is_parenthesized_new_callee);

    if should_expand_chain_left {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        format_left(f)
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
                format_with(format_left),
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

/// Format a binary expression with all operator-specific layout policies.
pub(crate) fn format_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &BinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_BINARY);

    // specialized logical and coalesce layout paths
    if try_format_trailing_coalesce(f, node_id, left, *operator, right)?
        || try_format_logical_right_prefix_block_comment(f, node_id, left, *operator, right)?
        || try_format_logical_right_prefix_line_comment(f, left, *operator, right)?
        || try_format_mixed_logical_precedence(f, left, *operator, right)?
        || try_format_logical_parenthesized_cases(f, node_id, left, *operator, right)?
    {
        return Ok(());
    }

    let layout_plan = analyze_binary_layout(f.context(), node_id, *operator);
    let operands = layout_plan.operands;
    let should_force_type_binary_expansion = layout_plan.should_force_type_binary_expansion;
    let is_type_union = layout_plan.is_type_union;
    let is_type_intersection = layout_plan.is_type_intersection;
    let binary_layout = layout_plan.layout;

    if matches!(binary_layout, BinaryLayout::CleanShortCircuit) {
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

    if matches!(binary_layout, BinaryLayout::LeadingPipeUnion) {
        format_leading_pipe_union(f, node_id, &operands)?;
        return Ok(());
    }

    if matches!(binary_layout, BinaryLayout::NullableUnionHug) {
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

    if matches!(binary_layout, BinaryLayout::StaticArgumentUnionHug) {
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

    if matches!(binary_layout, BinaryLayout::DestackIntersectionTrailing) {
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

    if matches!(binary_layout, BinaryLayout::CleanTypeShortCircuit) {
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
