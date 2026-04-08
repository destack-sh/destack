use crate::format::chain::{
    has_comment_between_expressions, is_assignment_chain_tail_lambda, is_chain_root,
    is_expression_chain, is_lambda_expression, transparent_inner_expression,
};
use crate::format::declaration::is_poorly_breakable_member_or_call_chain;
use crate::format::expression::{
    expression_has_prefix_comment_or_doc_annotation_in_left_spine,
    expression_is_trivial_inline_without_annotations, parenthesized_has_leading_inner_newline,
};
use crate::format::operator::flattened_binary_operand_count;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, AssignOperator, Declaration, Expression, LocalNodeId, NodeType,
    ScalarLiteral, TokenType,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{
    format_with, group, indent, indent_if_group_breaks, line_suffix_boundary,
    soft_line_break_or_space, soft_line_indent_or_space, space,
};
use destack_fir::write;
use destack_source::Span;

/// Return whether one expression has an own-line prefix annotation.
fn assign_expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let annotation = context.annotation(annotation_id);
            if !matches!(
                annotation.position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                return false;
            }

            context.annotation_starts_on_own_line(annotation_id)
        })
}

/// Decide whether an assignment can drop one parenthesized operand wrapper.
pub(crate) fn assignment_drops_parenthesized_operand_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
) -> bool {
    let drops_left_must_wrapper = matches!(
        parent_expression,
        Expression::Assign { left, .. } if *left == parenthesized_id
    ) && matches!(
        context.tree.get(inner_expression_id),
        Expression::Must { .. }
    );

    let drops_right_prefix_wrapper = matches!(
        parent_expression,
        Expression::Assign { right, .. } if *right == parenthesized_id
    ) && !context.has_annotation(parenthesized_id)
        && !parenthesized_has_leading_inner_newline(context, parenthesized_id, inner_expression_id)
        && expression_has_prefix_comment_or_doc_annotation_in_left_spine(
            context,
            inner_expression_id,
        );

    drops_left_must_wrapper || drops_right_prefix_wrapper
}

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

/// Return whether one expression has an inline prefix comment after an assignment operator.
pub(crate) fn assignment_rhs_has_inline_operator_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    assignment_rhs_has_inline_operator_prefix_annotation_style(context, expression_id, |_| true)
}

/// Return whether one expression has an inline slash prefix comment after an assignment operator.
fn assignment_rhs_has_inline_operator_prefix_slash_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    assignment_rhs_has_inline_operator_prefix_annotation_style(
        context,
        expression_id,
        |is_slash_style| is_slash_style,
    )
}

/// Return whether one expression has an inline prefix assignment-operator annotation matching one filter.
fn assignment_rhs_has_inline_operator_prefix_annotation_style(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    mut style_filter: impl FnMut(bool) -> bool,
) -> bool {
    let mut current_expression_id = transparent_inner_expression(context, expression_id);

    loop {
        let has_inline_prefix_comment = context
            .annotation_ids(current_expression_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                annotation_is_inline_assignment_operator_prefix_comment(
                    context,
                    annotation_id,
                    &mut style_filter,
                )
            });
        if has_inline_prefix_comment {
            return true;
        }

        let Some(next_expression_id) =
            next_assignment_left_spine_expression(context, current_expression_id)
        else {
            return false;
        };
        current_expression_id = next_expression_id;
    }
}

/// Return whether one annotation is an inline assignment-operator prefix comment.
fn annotation_is_inline_assignment_operator_prefix_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    style_filter: &mut impl FnMut(bool) -> bool,
) -> bool {
    let Some((comment_span, annotation_position, is_slash_style)) =
        assignment_operator_annotation_style(context, annotation_id)
    else {
        return false;
    };
    if !matches!(
        annotation_position,
        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
    ) {
        return false;
    }
    if !style_filter(is_slash_style) {
        return false;
    }

    let Some(previous_token) = context.annotation_previous_non_whitespace_token(annotation_id)
    else {
        return false;
    };
    if !is_assignment_operator_token(previous_token.token.ty) {
        return false;
    }

    let assignment_and_comment_share_line = context.file.is_same_line(
        previous_token.span.end.saturating_sub(1),
        comment_span.start,
    );
    if !assignment_and_comment_share_line {
        return false;
    }

    if context.annotation_starts_on_own_line(annotation_id) {
        return false;
    }

    if is_slash_style {
        true
    } else {
        !context.has_newline(comment_span)
            && context.annotation_next_token_is_on_same_line(annotation_id)
    }
}

/// Return node id, position, and style for one assignment-operator annotation.
fn assignment_operator_annotation_style(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<(Span, AnnotationPosition, bool)> {
    match context.annotation(annotation_id) {
        Annotation::Decorator { .. } => None,
    }
}

/// Return the next lhs-like expression on the assignment left spine.
fn next_assignment_left_spine_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(expression_id) {
        Expression::Parenthesized { expression } => Some(*expression),
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::TypeBinary { left, .. }
        | Expression::Binary { left, .. } => Some(*left),
        _ => None,
    }
}

/// Return whether one rhs expression is a class declaration shell.
fn expression_is_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(transparent_inner_expression(context, expression_id)),
        Expression::Declaration(declaration_id)
            if matches!(context.tree.get(*declaration_id), Declaration::Class { .. })
    )
}

/// Return whether one assignment operator has a slash line comment between left and right.
pub(crate) fn assignment_operator_has_line_comment_between(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left);
    let right_span = context.span(right);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };
    let comment_tokens = context.comment_tokens_intersecting_span(between_span);
    comment_tokens.into_iter().any(|comment_token| {
        if !matches!(
            comment_token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            return false;
        }

        context
            .previous_non_whitespace_token_before_span(comment_token.span)
            .is_some_and(|token| is_assignment_operator_token(token.token.ty))
    })
}

/// Walk left-linked assignment parents and return the outermost chain node.
pub(crate) fn left_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { left, .. } = parent_expression else {
            break;
        };
        if left.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Walk right-linked assignment parents and return the outermost chain node.
pub(crate) fn right_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { right, .. } = parent_expression else {
            break;
        };
        if right.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Return the direct assignment parent when current is the rhs.
pub(crate) fn right_assignment_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent(node_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let Expression::Assign { right, .. } = parent_expression else {
        return None;
    };
    if right.id != node_id.id {
        return None;
    }

    Some(parent_id)
}

// assignment shape thresholds
const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;

/// One OXC-style layout for one assignment-like shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssignmentLikeLayout {
    Fluid,
    BreakAfterOperator,
    NeverBreakAfterOperator,
    BreakLeftHandSide,
}

/// Write the right-hand side for one assignment-like layout.
pub(crate) fn write_assignment_like_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    layout: AssignmentLikeLayout,
    right: &impl Format<DestackFormatContext<'ast>>,
) -> FormatResult<()> {
    match layout {
        AssignmentLikeLayout::Fluid => {
            let group_id = f.group_id("assignment_like");

            write!(
                f,
                [
                    group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                    line_suffix_boundary(),
                    indent_if_group_breaks(right, group_id)
                ]
            )
        }
        AssignmentLikeLayout::BreakAfterOperator => {
            write!(f, [group(&soft_line_indent_or_space(right))])
        }
        AssignmentLikeLayout::NeverBreakAfterOperator => write!(f, [space(), right]),
        AssignmentLikeLayout::BreakLeftHandSide => write!(f, [space(), group(right)]),
    }
}

/// Select one layout for one assignment expression shell.
fn assignment_expression_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> FormatResult<AssignmentLikeLayout> {
    let context = f.context();

    let has_assignment_parent = context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Assign { .. }
            )
        });
    let has_left_assignment_parent =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                matches!(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { left, .. } if left.id == node_id.id
                )
            });
    let right_has_annotation = context.has_annotation(right);
    let assignment_has_newline = context.node_has_newline(node_id);
    let left_has_newline = context.node_has_newline(left);
    let right_has_newline = context.node_has_newline(right);
    let inner_right_id = transparent_inner_expression(context, right);
    let inner_right_expression = context.tree.get(inner_right_id);
    let right_is_binary = matches!(inner_right_expression, Expression::Binary { .. });
    let right_is_sequence = matches!(
        inner_right_expression,
        Expression::SequenceExpression { .. }
    );
    let right_is_assign = matches!(inner_right_expression, Expression::Assign { .. });
    let right_is_path_chain = matches!(
        inner_right_expression,
        Expression::QualifiedReference { path, .. } if path.segments.len() > 1
    );
    let right_is_chain = right_is_path_chain
        || is_expression_chain(context.tree, inner_right_id)
        || is_chain_root(context.tree, inner_right_id);
    let right_is_chain_tail_lambda =
        is_assignment_chain_tail_lambda(context, node_id, inner_right_id);
    let right_is_lambda = is_lambda_expression(context, inner_right_id);
    let right_is_self_breaking = right_is_binary
        || right_is_sequence
        || right_is_assign
        || right_is_chain
        || right_is_chain_tail_lambda
        || right_is_lambda;
    let right_has_prefix_annotation = context.has_prefix_annotation(right);
    let right_has_own_line_prefix_annotation =
        assign_expression_has_own_line_prefix_annotation(context, right);
    let right_has_assignment_operator_prefix_comment =
        assignment_rhs_has_inline_operator_prefix_comment(context, right)
            || assignment_operator_has_line_comment_between(context, left, right);
    let right_has_prefix_annotation_that_forces_operator_break =
        right_has_prefix_annotation && !right_has_assignment_operator_prefix_comment;
    let right_has_between_comment = has_comment_between_expressions(context, left, right);
    let is_string_literal = matches!(
        inner_right_expression,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );
    let chain_root_is_current = left_assignment_chain_root(context, node_id).id == node_id.id;
    let right_chain_root_id = right_assignment_chain_root(context, node_id);
    let right_assignment_chain_is_multiline = has_assignment_parent && {
        let right_parent_is_long = right_assignment_parent(context, node_id)
            .is_some_and(|parent_id| context.node_has_newline(parent_id));
        right_parent_is_long || context.node_has_newline(right_chain_root_id)
    };
    let right_is_multiline_binary = right_is_binary && {
        let binary_operand_count = match inner_right_expression {
            Expression::Binary { operator, .. } => {
                flattened_binary_operand_count(context, inner_right_id, *operator)
            }
            _ => 0,
        };
        binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD
            && (right_has_between_comment || right_has_newline)
    };
    let right_is_inline_atomic =
        expression_is_trivial_inline_without_annotations(context, inner_right_id);
    let right_is_class_declaration = expression_is_class_declaration(context, right);

    // string literals are atomic: never break at `=`
    if is_string_literal {
        return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
    }

    // keep assignment operator prefix comments with the operator
    if right_has_assignment_operator_prefix_comment {
        let right_has_inline_operator_slash_comment =
            assignment_rhs_has_inline_operator_prefix_slash_comment(f.context(), right);
        if !right_has_inline_operator_slash_comment
            && !right_has_newline
            && !right_has_own_line_prefix_annotation
        {
            return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
        }

        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    // non-inline comments between `=` and rhs always break at the operator
    if right_has_prefix_annotation_that_forces_operator_break
        || right_has_between_comment
        || right_has_own_line_prefix_annotation
    {
        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    // class expression shells stay attached to the operator
    if right_is_class_declaration {
        return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
    }

    // left associative assignment chains: keep inner chain steps inline
    if has_left_assignment_parent && !right_has_newline && chain_root_is_current {
        return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
    }

    // expanded right-associative assignment chains should break at each operator
    if right_assignment_chain_is_multiline {
        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    // break long binary rhs values after the operator
    if right_is_multiline_binary {
        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    if right_is_chain && is_poorly_breakable_member_or_call_chain(f, inner_right_id) {
        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    if matches!(inner_right_expression, Expression::ObjectExpression { .. })
        && !right_has_newline
        && !right_has_annotation
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
        && !right_has_own_line_prefix_annotation
    {
        return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
    }

    // rhs-managed layout cases
    if right_is_self_breaking {
        if right_is_chain {
            if right_has_newline {
                return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
            }

            return Ok(AssignmentLikeLayout::Fluid);
        }

        if right_is_assign {
            return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
        }

        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    if left_has_newline || assignment_has_newline || right_has_newline {
        return Ok(AssignmentLikeLayout::BreakAfterOperator);
    }

    // keep atomic rhs values inline when there are no break signals
    if right_is_inline_atomic
        && !right_is_chain
        && !left_has_newline
        && !right_has_newline
        && !right_has_annotation
        && !right_has_prefix_annotation_that_forces_operator_break
        && !right_has_between_comment
    {
        return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
    }

    Ok(AssignmentLikeLayout::Fluid)
}

/// Write one assignment expression with one chosen layout.
fn write_assignment_expression_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    layout: AssignmentLikeLayout,
) -> FormatResult<()> {
    let has_left_postfix = f.context().has_postfix_annotation(left);
    let left = format_with(|f| {
        write!(f, [left])?;

        if !has_left_postfix {
            write!(f, [space()])?;
        }

        write!(f, [operator])
    });

    if layout == AssignmentLikeLayout::BreakLeftHandSide {
        write!(f, [left])?;
    } else {
        write!(f, [group(&left)])?;
    }

    write_assignment_like_right(f, layout, &right)
}

/// Format an assignment expression with one selected layout.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let layout = assignment_expression_layout(f, node_id, left, right)?;

    write_assignment_expression_layout(f, left, operator, right, layout)
}
