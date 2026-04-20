use super::binary::is_logical_binary_operator;
use crate::format::chain::{
    assignment_like_parent, has_own_line_or_multiline_comment_between_expressions,
    is_assignment_chain_tail_lambda, transparent_inner_expression,
};
use crate::format::expression::ExpressionLeftSide;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    AssignOperator, Comment, Declaration, DecoratorPosition, Expression, IfCondition, IfKind, Key,
    LocalNodeId, Name, NodeType, Property, ScalarLiteral, TokenType,
};
use destack_fir::format::{
    Buffer, Format, FormatNode as FirFormatNode, FormatNodes, FormatResult,
    Formatter as FirFormatter, VecBuffer,
};
use destack_fir::prelude::{
    format_with, group, indent, indent_if_group_breaks, line_suffix_boundary,
    soft_line_break_or_space, soft_line_indent_or_space, space,
};
use destack_fir::write;
use destack_source::Span;

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

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
                annotation.position,
                DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
            ) {
                return false;
            }

            context.annotation_starts_on_own_line(annotation_id)
        })
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

/// Return whether one expression has an inline prefix assignment-operator annotation matching one filter.
fn assignment_rhs_has_inline_operator_prefix_annotation_style(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    mut style_filter: impl FnMut(bool) -> bool,
) -> bool {
    let mut left_side = Some(ExpressionLeftSide::new(transparent_inner_expression(
        context,
        expression_id,
    )));

    while let Some(current_left_side) = left_side {
        let current_expression_id = current_left_side.expression_id();

        let has_inline_prefix_comment =
            assignment_rhs_operator_comment_nodes(context, current_expression_id)
                .into_iter()
                .any(|comment| {
                    let Some(previous_token) =
                        context.previous_non_trivia_token_before_span(comment.span)
                    else {
                        return false;
                    };
                    if !is_assignment_operator_token(previous_token.token.ty) {
                        return false;
                    }

                    let assignment_and_comment_share_line = context.file.is_same_line(
                        previous_token.span.end.saturating_sub(1),
                        comment.span.start,
                    );
                    if !assignment_and_comment_share_line {
                        return false;
                    }

                    let is_slash_style = comment.is_line();
                    if !style_filter(is_slash_style) {
                        return false;
                    }

                    if is_slash_style {
                        return true;
                    }

                    !context.has_newline(comment.span)
                        && !context.span_has_newline_before_next_non_whitespace_token(comment.span)
                });
        if has_inline_prefix_comment {
            return true;
        }

        left_side = current_left_side.left(context);
    }

    false
}

/// Return whether one rhs expression is a class declaration shell.
fn expression_is_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(transparent_inner_expression(context, expression_id)),
        Expression::Declaration(declaration_id)
            if matches!(context.tree.get(*declaration_id), Declaration::Class(_))
    )
}

/// Return whether one rhs expression is a CommonJS `require(...)` call.
pub(crate) fn expression_is_commonjs_require_call(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    let Expression::Call { left, .. } = context.tree.get(expression_id) else {
        return false;
    };

    matches!(
        context.tree.get(transparent_inner_expression(context, *left)),
        Expression::Identifier { name } if context.strings.get(*name) == "require"
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

/// Return comments between one assignment operator and rhs expression.
fn assignment_rhs_operator_comment_nodes(
    context: &DestackFormatContext<'_>,
    right: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let right_span = context.span(right);
    let right_token_start = context
        .first_non_trivia_token_in_span(right_span)
        .map_or(right_span.start, |token| token.span.start);
    let Some(previous_token) = context.previous_non_trivia_token_before_span(Span::new(
        right_span.file,
        right_token_start,
        right_token_start,
    )) else {
        return Vec::new();
    };

    if !is_assignment_operator_token(previous_token.token.ty)
        || previous_token.span.file != right_span.file
        || previous_token.span.end >= right_token_start
    {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(previous_token.span.end, right_token_start)
            .to_vec()
    }
}

/// Buffer the assignment left-hand side so layout can inspect the formatted width first.
fn buffer_assignment_left<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);

    // left side
    write!(formatter, [left])?;

    let nodes = buffer.into_vec();
    let is_short = nodes.single_line_width().is_some_and(|width| {
        width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
    });
    let may_break = nodes.may_directly_break();

    Ok((nodes, is_short, may_break))
}

/// One layout for one assignment-like shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssignmentLikeLayout {
    /// Break the right-hand side only when it forces the outer group to expand.
    Fluid,

    /// Break after the operator and indent the right-hand side as one unit.
    BreakAfterOperator,

    /// Keep the operator and right-hand side on the same line.
    NeverBreakAfterOperator,

    /// Break the left-hand side first and then group the right-hand side independently.
    BreakLeftHandSide,

    /// Keep a chained assignment head attached to the following assignment shell.
    Chain,

    /// Indent the final right-hand side of one eligible assignment chain.
    ChainTail,

    /// Keep one arrow-function tail attached to the final chain operator.
    ChainTailArrowFunction,
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
        AssignmentLikeLayout::Chain => write!(f, [soft_line_break_or_space(), right]),
        AssignmentLikeLayout::ChainTail => write!(f, [soft_line_indent_or_space(right)]),
        AssignmentLikeLayout::ChainTailArrowFunction => write!(f, [space(), right]),
    }
}

/// Return whether one rhs shape should prefer breaking after the operator.
pub(crate) fn assignment_rhs_prefers_break_after_operator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    right: LocalNodeId<Expression>,
) -> bool {
    let context = f.context();
    let right = transparent_inner_expression(context, right);

    // comments
    for comment in context
        .comments()
        .comments_before_iter(context.span(right).start)
    {
        if comment.preceded_by_newline() && comment.followed_by_newline() {
            return true;
        }
    }

    match context.tree.get(right) {
        // assignment chains break after `=`
        Expression::Assign {
            right: nested_right,
            ..
        } => matches!(
            context
                .tree
                .get(transparent_inner_expression(context, *nested_right)),
            Expression::Assign { .. }
        ),

        // binary-like rhs values first break after `=`
        Expression::Binary { .. } | Expression::SequenceExpression { .. } => true,

        // ternary rhs values only break after `=` when the test is binary-like
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            ..
        } => match condition {
            IfCondition::Expression { condition } => {
                let condition = transparent_inner_expression(context, *condition);

                match context.tree.get(condition) {
                    Expression::Binary {
                        operator, right, ..
                    } => {
                        if !is_logical_binary_operator(*operator) {
                            return true;
                        }

                        let logical_right = transparent_inner_expression(context, *right);
                        let right_stays_inline = match context.tree.get(logical_right) {
                            Expression::ObjectExpression { properties, .. } => {
                                !properties.is_empty()
                            }
                            Expression::ArrayExpression { elements } => !elements.is_empty(),
                            Expression::TreeExpression { .. } => true,
                            _ => false,
                        };

                        !right_stays_inline
                    }
                    _ => false,
                }
            }
            IfCondition::Let { .. } => false,
        },

        _ => false,
    }
}

/// Return whether one object field is shorthand.
fn property_field_is_shorthand(
    context: &DestackFormatContext<'_>,
    key: Key,
    value: LocalNodeId<Expression>,
) -> bool {
    matches!(
        (key, context.tree.get(value)),
        (
            Key::Name(Name::Identifier(key_name)),
            Expression::Identifier { name: value_name },
        ) if key_name == *value_name
    )
}

/// Return whether one assignment target is complex enough to break the left side.
fn assignment_target_is_complex_destructuring(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
) -> bool {
    let left = transparent_inner_expression(context, left);
    let Expression::ObjectExpression { properties, .. } = context.tree.get(left) else {
        return false;
    };

    if properties.len() <= 2 {
        return false;
    }

    properties
        .iter()
        .copied()
        .any(|property_id| match context.tree.get(property_id) {
            Property::Field { key, value } => !property_field_is_shorthand(context, *key, *value),
            Property::Method { .. } | Property::Error => true,
            Property::Spread { .. } => false,
        })
}

/// Return whether one rhs expression stays attached to the assignment operator.
fn assignment_rhs_is_compact(
    context: &DestackFormatContext<'_>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right = transparent_inner_expression(context, right);
    let right_expression = context.tree.get(right);

    matches!(
        right_expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Boolean(_)
                | ScalarLiteral::Integer(_)
                | ScalarLiteral::Bigint(_)
                | ScalarLiteral::Float(_)
                | ScalarLiteral::String(_)
        ) | Expression::TemplateExpression { .. }
            | Expression::TaggedTemplateExpression { .. }
    ) || expression_is_class_declaration(context, right)
}

/// Return whether one assignment rhs carries leading trivia that forces break-after-operator.
fn assignment_expression_rhs_has_forcing_leading_trivia(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let has_prefix_annotation = context.has_prefix_annotation(right);
    let has_between_comment =
        has_own_line_or_multiline_comment_between_expressions(context, left, right);
    let has_own_line_prefix_annotation =
        assign_expression_has_own_line_prefix_annotation(context, right);

    has_prefix_annotation || has_between_comment || has_own_line_prefix_annotation
}

/// Return one explicit chain layout for an eligible assignment shell.
fn assignment_expression_chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> Option<AssignmentLikeLayout> {
    let right = transparent_inner_expression(context, right);
    let right_is_tail = !matches!(context.tree.get(right), Expression::Assign { .. });
    let Some((parent_type, parent_id)) = assignment_like_parent(context, node_id) else {
        return None;
    };

    let is_eligible = match parent_type {
        NodeType::Declarator => !right_is_tail,
        NodeType::Expression => {
            let parent_id = LocalNodeId::<Expression>::new(parent_id);
            if !matches!(context.tree.get(parent_id), Expression::Assign { .. }) {
                return None;
            }

            !right_is_tail || assignment_like_parent(context, parent_id).is_some()
        }
        _ => false,
    };

    if !is_eligible {
        return None;
    }

    if right_is_tail {
        if is_assignment_chain_tail_lambda(context, node_id, right) {
            return Some(AssignmentLikeLayout::ChainTailArrowFunction);
        }

        return Some(AssignmentLikeLayout::ChainTail);
    }

    Some(AssignmentLikeLayout::Chain)
}

/// Select one layout for one assignment expression shell.
fn assignment_expression_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
    is_left_short: bool,
    left_may_break: bool,
) -> AssignmentLikeLayout {
    // assignment chains
    if let Some(layout) = assignment_expression_chain_layout(f.context(), node_id, right) {
        return layout;
    }

    // compact CommonJS require calls stay attached to `=`
    if expression_is_commonjs_require_call(f.context(), right)
        && !f
            .context()
            .comments()
            .has_leading_own_line_comment(f.context().span(right).start)
    {
        return AssignmentLikeLayout::NeverBreakAfterOperator;
    }

    // complex destructuring breaks its left side first
    if assignment_target_is_complex_destructuring(f.context(), left) {
        return AssignmentLikeLayout::BreakLeftHandSide;
    }

    // operator-bound trivia and rhs pressure break after the operator
    if assignment_rhs_has_inline_operator_prefix_comment(f.context(), right)
        || assignment_operator_has_line_comment_between(f.context(), left, right)
        || assignment_expression_rhs_has_forcing_leading_trivia(f.context(), left, right)
        || assignment_rhs_prefers_break_after_operator(f, right)
    {
        return AssignmentLikeLayout::BreakAfterOperator;
    }

    // compact rhs
    if !left_may_break && (is_left_short || assignment_rhs_is_compact(f.context(), right)) {
        return AssignmentLikeLayout::NeverBreakAfterOperator;
    }

    AssignmentLikeLayout::Fluid
}

/// Write one assignment expression with one chosen layout.
fn write_assignment_expression_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left_nodes: Vec<FirFormatNode>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
    layout: AssignmentLikeLayout,
) -> FormatResult<()> {
    let has_left_postfix = f.context().has_postfix_annotation(left);
    let left_nodes = f.intern_vec(left_nodes);
    let formatted_left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if let Some(left_nodes) = &left_nodes {
            f.write_node(left_nodes.clone());
        }

        Ok(())
    });
    let formatted_operator = format_with(|f| {
        if !has_left_postfix {
            write!(f, [space()])?;
        }

        write!(f, [operator])?;
        Ok(())
    });

    let content = format_with(|f| {
        if layout == AssignmentLikeLayout::BreakLeftHandSide {
            write!(f, [formatted_left])?;
        } else {
            write!(f, [group(&formatted_left)])?;
        }

        write!(f, [formatted_operator])?;

        write_assignment_like_right(f, layout, &right)
    });

    match layout {
        AssignmentLikeLayout::Chain
        | AssignmentLikeLayout::ChainTail
        | AssignmentLikeLayout::ChainTailArrowFunction => write!(f, [content]),
        _ => write!(f, [group(&content)]),
    }
}

/// Format an assignment expression with one selected layout.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let (left_nodes, is_left_short, left_may_break) = buffer_assignment_left(f, left)?;
    let layout =
        assignment_expression_layout(f, node_id, left, right, is_left_short, left_may_break);
    write_assignment_expression_layout(f, left_nodes, left, operator, right, layout)
}
