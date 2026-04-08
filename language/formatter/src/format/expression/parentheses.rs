use super::{
    argument_drops_parenthesized_value_wrapper, declarator_drops_parenthesized_value_wrapper,
    format_expression, format_inline_ternary_expression,
    postfix_continuation_requires_parenthesized_object_wrapper,
    should_hoist_parenthesized_inner_cast_prefix_comments,
    write_expression_without_prefix_annotations,
};
use crate::format::annotation::{
    format_raw_comment, infix_or_postfix_annotations, prefix_annotations, write_raw_comment_slice,
};
use crate::format::call::call_drops_parenthesized_callee_wrapper;
use crate::format::chain::transparent_inner_expression;
use crate::format::context::ParenthesizedExpressionView;
use crate::format::directive::node_has_ignore_directive;
use crate::format::operator::{
    assignment_drops_parenthesized_operand_wrapper, binary_keeps_unary_left_parenthesized_wrapper,
    format_binary_expression, should_drop_parenthesized_type_expression,
};
use crate::format::tree::format_parenthesized_tree_expression;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Argument, BinaryOperator, Comment, Expression, IfKind, LocalNodeId,
    NodeType, TokenType,
};
use destack_fir::format::{BestFittingMode, Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, soft_block_indent, space, token,
};
use destack_fir::{best_fitting, format_args, write};
use destack_source::Span;

/// One preserved parenthesized-expression layout.
enum PreservedParenthesizedLayout {
    TypeCastComment,
    TreeExpression {
        arguments: Option<Vec<LocalNodeId<Argument>>>,
        elements: Option<Vec<LocalNodeId<Argument>>>,
        has_leading_inner_trivia: bool,
    },
    HoistedCastPrefix,
    DecoratorOrAssignment,
    NewlineOnly,
    LeadingComments,
    Plain,
}

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

/// Return whether one parenthesized wrapper has explicit `(` and `)` delimiter tokens.
pub(crate) fn parenthesized_has_explicit_delimiters(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    _inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    ParenthesizedExpressionView::from_node(context, parenthesized_id).is_some()
}

/// Return whether one expression is the parenthesized node of a type-cast comment wrapper.
pub(crate) fn is_type_cast_comment_node(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Parenthesized { expression } = context.tree.get(node_id) else {
        return false;
    };
    if !parenthesized_has_explicit_delimiters(context, node_id, *expression) {
        return false;
    }

    let node_span = context.span(node_id);
    let mut cursor_start = node_span.start;

    for comment_span in context.comment_spans.iter().rev().copied() {
        if comment_span.file != node_span.file || comment_span.end > cursor_start {
            continue;
        }

        let between_span = Span::new(node_span.file, comment_span.end, cursor_start);
        if context.has_non_whitespace_content(between_span) {
            return false;
        }

        if context.comment_token_type_at_span(comment_span) == Some(TokenType::DocBlockComment) {
            return true;
        }

        cursor_start = comment_span.start;
    }

    false
}

/// Format one parenthesized type-cast comment wrapper.
pub(crate) fn format_type_cast_comment_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
    trailing_inner_line_comments: &[Comment],
) -> FormatResult<bool> {
    if !is_type_cast_comment_node(f.context(), node_id) {
        return Ok(false);
    }

    write!(f, [prefix_annotations(f.context(), node_id)])?;

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [expression_id])?;

        for comment in trailing_inner_line_comments {
            write!(f, [hard_line_break()])?;
            format_raw_comment(f, *comment)?;
        }

        Ok(())
    });
    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&format_inner),
            token(")")
        ])]
    )?;

    Ok(true)
}

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
pub(crate) fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if is_type_cast_comment_node(context, node_id) {
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

/// Return whether one expression or declaration carries decorator prefix annotations.
fn inner_expression_has_decorator_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_has_decorator =
        context
            .annotation_ids(expression_id)
            .iter()
            .any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Decorator {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            });

    if expression_has_decorator {
        return true;
    }

    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    context
        .annotation_ids(*declaration_id)
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Decorator {
                    position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                    ..
                }
            )
        })
}

/// Return whether one parenthesized expression appears in an assignment value position.
fn parenthesized_is_in_assignment_value_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            return false;
        };

        match parent_type {
            NodeType::Expression => {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                let parent_expression = context.tree.get(parent_expression_id);
                if matches!(
                    parent_expression,
                    Expression::Assign { right, .. } if *right == current_id
                ) {
                    return true;
                }

                if let Expression::If {
                    kind: IfKind::Ternary,
                    condition,
                    then_expression,
                    else_expression,
                } = parent_expression
                {
                    let is_ternary_test = matches!(
                        condition,
                        destack_ast::IfCondition::Expression { condition } if *condition == current_id
                    );
                    let is_ternary_branch = *then_expression == current_id
                        || else_expression
                            .is_some_and(|else_expression| else_expression == current_id);

                    if is_ternary_branch || !is_ternary_test {
                        return false;
                    }
                }

                current_id = parent_expression_id;
            }
            NodeType::Declarator => return true,
            _ => return false,
        }
    }
}

/// Format one dropped parenthesized expression wrapper.
fn format_dropped_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let normalized_inner_id = transparent_inner_expression(f.context(), expression_id);
    if let Expression::Binary {
        left,
        operator: operator @ (BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd),
        right,
    } = f.context().tree.get(normalized_inner_id)
    {
        return format_binary_expression(f, node_id, *left, operator, *right);
    }

    write!(f, [expression_id])
}

/// Format one hoisted cast-prefix parenthesized expression.
fn format_hoisted_cast_prefix_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let inner_is_ignored = node_has_ignore_directive(f.context(), expression_id);
    let format_inner_without_prefix = format_with(|f| {
        format_expression(
            f,
            expression_id,
            f.context().tree.get(expression_id),
            inner_is_ignored,
        )?;
        write!(
            f,
            [infix_or_postfix_annotations(f.context(), expression_id)]
        )?;
        Ok(())
    });

    write!(f, [prefix_annotations(f.context(), expression_id)])?;
    write!(
        f,
        [group(&format_args![
            token("("),
            format_inner_without_prefix,
            token(")")
        ])]
    )
}

/// Format one preserved wrapper for decorator or assignment-comment cases.
fn format_decorator_or_assignment_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(
        f,
        [
            token("("),
            block_indent(&group(&expression_id).should_expand(true)),
            hard_line_break(),
            token(")")
        ]
    )
}

/// Format one preserved wrapper that only keeps a leading newline.
fn format_newline_only_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let trailing_inner_comment_nodes =
        parenthesized_trailing_inner_comments(f.context(), node_id, expression_id);
    let format_inline_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if matches!(
            f.context().tree.get(expression_id),
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            format_inline_ternary_expression(f, expression_id)
        } else {
            write!(f, [group(&expression_id)])
        }
    });

    write!(
        f,
        [best_fitting![
            format_args![
                token("("),
                format_inline_expression,
                format_with(|f| write_raw_comment_slice(f, &trailing_inner_comment_nodes)),
                token(")")
            ],
            format_args![
                token("("),
                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [group(&expression_id)])?;

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
        ]
        .with_mode(BestFittingMode::AllLines)]
    )
}

/// Format one preserved wrapper with comments between `(` and the inner expression.
fn format_leading_comment_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let leading_inner_comments =
        parenthesized_leading_inner_comments(f.context(), node_id, expression_id);
    let format_inner = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        for (comment_index, comment_id) in leading_inner_comments.iter().enumerate() {
            if comment_index > 0 {
                write!(f, [hard_line_break()])?;
            }

            format_raw_comment(f, *comment_id)?;
            write!(f, [hard_line_break()])?;
        }

        write!(
            f,
            [group(&format_with(|f| {
                write_expression_without_prefix_annotations(f, expression_id)
            }))
            .should_expand(true)]
        )
    });

    write!(
        f,
        [
            token("("),
            block_indent(&format_inner),
            hard_line_break(),
            token(")")
        ]
    )
}

/// Write comments that belong immediately after one preserved closing `)`.
fn write_parenthesized_boundary_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    _expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let boundary_comments = ParenthesizedExpressionView::from_node(f.context(), node_id)
        .map(ParenthesizedExpressionView::boundary_comments)
        .unwrap_or_default();
    for comment in boundary_comments {
        write!(f, [space()])?;
        format_raw_comment(f, comment)?;
    }

    Ok(())
}

/// Format one preserved parenthesized expression wrapper.
fn format_preserved_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let trailing_inner_comment_nodes =
        parenthesized_trailing_inner_comments(f.context(), node_id, expression_id);
    let trailing_inner_line_comments = trailing_inner_comment_nodes
        .iter()
        .copied()
        .filter(|comment| comment.is_line())
        .collect::<Vec<_>>();
    let layout = preserved_parenthesized_layout(f.context(), node_id, expression_id);

    match layout {
        PreservedParenthesizedLayout::TypeCastComment => {
            format_type_cast_comment_node(
                f,
                node_id,
                expression_id,
                &trailing_inner_line_comments,
            )?;
        }
        PreservedParenthesizedLayout::TreeExpression {
            arguments,
            elements,
            has_leading_inner_trivia,
        } => {
            format_parenthesized_tree_expression(
                f,
                node_id,
                expression_id,
                &arguments,
                &elements,
                has_leading_inner_trivia,
                &trailing_inner_comment_nodes,
            )?;
        }
        PreservedParenthesizedLayout::HoistedCastPrefix => {
            format_hoisted_cast_prefix_parenthesized_expression(f, expression_id)?;
        }
        PreservedParenthesizedLayout::DecoratorOrAssignment => {
            format_decorator_or_assignment_parenthesized_expression(f, expression_id)?;
        }
        PreservedParenthesizedLayout::NewlineOnly => {
            format_newline_only_parenthesized_expression(f, node_id, expression_id)?;
        }
        PreservedParenthesizedLayout::LeadingComments => {
            format_leading_comment_parenthesized_expression(f, node_id, expression_id)?;
        }
        PreservedParenthesizedLayout::Plain => {
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

/// Decide how one preserved parenthesized expression should render.
fn preserved_parenthesized_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> PreservedParenthesizedLayout {
    let inner_expression = context.tree.get(expression_id);
    let parenthesized_view = ParenthesizedExpressionView::from_node(context, node_id);
    let has_leading_inner_trivia =
        parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia);
    let has_leading_inner_comments =
        parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_comments);
    let has_leading_inner_newline =
        parenthesized_view.is_some_and(ParenthesizedExpressionView::has_leading_inner_newline);
    let has_inner_decorator_prefix_annotation =
        inner_expression_has_decorator_prefix_annotation(context, expression_id);
    let is_in_assignment_value_context =
        parenthesized_is_in_assignment_value_context(context, node_id);
    let prefers_inline_scalar_comment_wrapper =
        matches!(inner_expression, Expression::ScalarLiteral(_))
            && has_leading_inner_comments
            && !has_leading_inner_newline
            && !context.node_has_newline(expression_id);
    let preserve_newline_only_wrapper = has_leading_inner_newline && !has_leading_inner_comments;

    // type-cast comment wrappers own their trailing inner line comments
    if is_type_cast_comment_node(context, node_id) {
        return PreservedParenthesizedLayout::TypeCastComment;
    }

    // tree expressions keep the specialized tree formatter
    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = inner_expression
    {
        return PreservedParenthesizedLayout::TreeExpression {
            arguments: arguments.clone(),
            elements: elements.clone(),
            has_leading_inner_trivia,
        };
    }

    // cast-prefix comments can move out unless assignment ownership blocks it
    if !is_in_assignment_value_context
        && should_hoist_parenthesized_inner_cast_prefix_comments(context, node_id, expression_id)
    {
        return PreservedParenthesizedLayout::HoistedCastPrefix;
    }

    // decorators and assignment-comment wrappers force the expanded shell
    if has_inner_decorator_prefix_annotation
        || (is_in_assignment_value_context && has_leading_inner_comments)
    {
        return PreservedParenthesizedLayout::DecoratorOrAssignment;
    }

    // pure leading newline wrappers get the lighter preserved shell
    if preserve_newline_only_wrapper && !prefers_inline_scalar_comment_wrapper {
        return PreservedParenthesizedLayout::NewlineOnly;
    }

    // leading comments own the preserved comment shell
    if has_leading_inner_comments && !prefers_inline_scalar_comment_wrapper {
        return PreservedParenthesizedLayout::LeadingComments;
    }

    PreservedParenthesizedLayout::Plain
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

    format_preserved_parenthesized_expression(f, node_id, expression_id)?;
    write_parenthesized_boundary_comments(f, node_id, expression_id)
}
