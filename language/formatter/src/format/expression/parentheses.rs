use super::object::parenthesized_assignment_target_prefers_expanded_layout;
use super::{
    argument_drops_parenthesized_value_wrapper, declarator_drops_parenthesized_value_wrapper,
    format_expression, postfix_continuation_requires_parenthesized_object_wrapper,
    should_hoist_parenthesized_inner_cast_prefix_comments,
    statement_drops_parenthesized_expression_wrapper,
};
use crate::format::call::call_drops_parenthesized_callee_wrapper;
use crate::format::directive::node_has_ignore_directive;
use crate::format::operator::{
    assignment_drops_parenthesized_operand_wrapper, binary_keeps_unary_left_parenthesized_wrapper,
    format_binary_expression, normalize_parenthesized_type_grouping_inner_expression,
    parenthesized_type_expression_prefers_soft_block_layout,
    should_drop_parenthesized_type_expression,
};
use crate::format::tree::format_parenthesized_tree_expression;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, BinaryOperator, Comment, CommentStyle, Expression, IfKind, LocalNodeId,
    NodeType, TokenType,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, soft_block_indent, space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

/// Collect postfix star comments from an inner expression that should render after `)`.
pub(crate) fn parenthesized_boundary_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return Vec::new();
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return Vec::new();
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);
    if parenthesized_span.file != inner_span.file || inner_span.end >= parenthesized_span.end {
        return Vec::new();
    }

    let boundary_span = Span::new(
        parenthesized_span.file,
        inner_span.end,
        parenthesized_span.end,
    );
    if context
        .comments_in_range(boundary_span.start, boundary_span.end)
        .is_empty()
    {
        return Vec::new();
    }

    let comment_trivia = context.tree.comment_trivia();
    let first_relevant_index =
        comment_trivia.partition_point(|comment_trivia| comment_trivia.span.end < inner_span.end);

    let mut comments: Vec<(u32, LocalNodeId<Comment>)> = Vec::new();
    for comment_trivia in comment_trivia[first_relevant_index..].iter().copied() {
        if comment_trivia.span.start > parenthesized_span.end {
            break;
        }
        if context.tree.get(comment_trivia.comment).style != CommentStyle::Star {
            continue;
        }
        if comment_trivia.span.start < inner_span.end
            || comment_trivia.span.end > parenthesized_span.end
        {
            continue;
        }

        if context
            .next_non_whitespace_token_after_span(comment_trivia.span)
            .is_none_or(|token| token.token.ty != TokenType::CloseParenthesis)
        {
            continue;
        }
        comments.push((comment_trivia.span.start, comment_trivia.comment));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment_id)| comment_id)
        .collect()
}

/// Collect comments between `(` and the inner expression.
pub(crate) fn parenthesized_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    if !parenthesized_has_explicit_delimiters(context, parenthesized_id, inner_expression_id) {
        return Vec::new();
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);
    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return Vec::new();
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    if context
        .comments_in_range(leading_span.start, leading_span.end)
        .is_empty()
    {
        return Vec::new();
    }

    let comment_trivia = context.tree.comment_trivia();
    let first_relevant_index = comment_trivia
        .partition_point(|comment_trivia| comment_trivia.span.end < leading_span.start);

    let mut comments: Vec<(u32, LocalNodeId<Comment>)> = Vec::new();
    for comment_trivia in comment_trivia[first_relevant_index..].iter().copied() {
        if comment_trivia.span.file != leading_span.file
            || comment_trivia.span.start >= leading_span.end
        {
            break;
        }
        if comment_trivia.span.start < leading_span.start
            || comment_trivia.span.end > leading_span.end
        {
            continue;
        }

        comments.push((comment_trivia.span.start, comment_trivia.comment));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment_id)| comment_id)
        .collect()
}

/// Return whether one parenthesized wrapper has explicit `(` and `)` delimiter tokens.
pub(crate) fn parenthesized_has_explicit_delimiters(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    if parenthesized_span.file != inner_span.file
        || parenthesized_span.start >= inner_span.start
        || inner_span.end >= parenthesized_span.end
    {
        return false;
    }

    let Some(open_parenthesis) = context.previous_non_whitespace_token_before_span(inner_span)
    else {
        return false;
    };
    if open_parenthesis.token.ty != TokenType::OpenParenthesis
        || open_parenthesis.span.file != parenthesized_span.file
        || open_parenthesis.span.start < parenthesized_span.start
        || open_parenthesis.span.end > parenthesized_span.end
    {
        return false;
    }

    let Some(close_parenthesis) = context.next_non_whitespace_token_after_span(inner_span) else {
        return false;
    };
    if close_parenthesis.token.ty != TokenType::CloseParenthesis
        || close_parenthesis.span.file != parenthesized_span.file
        || close_parenthesis.span.start < parenthesized_span.start
        || close_parenthesis.span.end > parenthesized_span.end
    {
        return false;
    }

    true
}

/// Return whether source contains leading trivia between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, true)
}

/// Return whether source contains leading comments between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, false)
}

/// Return whether source contains a newline between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_newline(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !parenthesized_has_explicit_delimiters(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    context.has_newline(leading_span)
}

/// Return whether source contains leading comment or newline trivia between `(` and inner.
fn parenthesized_has_leading_inner_pattern(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    include_newline: bool,
) -> bool {
    if !parenthesized_has_explicit_delimiters(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    if include_newline && context.has_newline(leading_span) {
        return true;
    }

    !context
        .comments_in_range(leading_span.start, leading_span.end)
        .is_empty()
}

/// Return whether source contains line comments between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_line_comment(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !parenthesized_has_explicit_delimiters(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    context
        .line_comment_spans
        .iter()
        .copied()
        .any(|comment_span| {
            comment_span.file == leading_span.file
                && comment_span.start < leading_span.end
                && comment_span.end > leading_span.start
        })
}

/// Return whether a parenthesized wrapper is immediately preceded by a closure-style type-cast comment.
pub(crate) fn parenthesized_has_leading_type_cast_comment(
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

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
pub(crate) fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if parenthesized_has_leading_type_cast_comment(context, node_id) {
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

    let is_statement_wrapper =
        matches!(parent_expression, Expression::Statement(inner_id) if inner_id.id == node_id.id);
    if is_statement_wrapper {
        return statement_drops_parenthesized_expression_wrapper(
            context,
            node_id,
            inner_expression_id,
        );
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

/// Format a parenthesized primary expression.
pub(crate) fn format_primary_parenthesized_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let expression = &expression_id;
    let inner_expression = tree.get(expression_id);
    let should_drop_parentheses =
        should_drop_parenthesized_expression_wrapper(f.context(), node_id, expression_id);

    // dropped wrapper
    if should_drop_parentheses {
        let normalized_inner_id =
            normalize_parenthesized_type_grouping_inner_expression(f.context(), expression_id);
        if let Expression::Binary {
            left,
            operator: operator @ (BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd),
            right,
        } = f.context().tree.get(normalized_inner_id)
        {
            return format_binary_expression(f, node_id, *left, operator, *right);
        }

        write!(f, [expression_id])?;
    }
    // preserved wrapper
    else {
        let has_parenthesized_leading_inner_trivia =
            parenthesized_has_leading_inner_trivia(f.context(), node_id, expression_id);
        let has_parenthesized_leading_inner_comments =
            parenthesized_has_leading_inner_comments(f.context(), node_id, expression_id);
        let has_parenthesized_leading_inner_newline =
            parenthesized_has_leading_inner_newline(f.context(), node_id, expression_id);
        let has_inner_decorator_prefix_annotation = {
            let expression_has_decorator =
                f.context()
                    .annotation_ids(expression_id)
                    .iter()
                    .any(|annotation_id| {
                        matches!(
                            f.context().annotation(*annotation_id),
                            Annotation::Decorator {
                                position: AnnotationPosition::BlockPrefix
                                    | AnnotationPosition::LinePrefix,
                                ..
                            }
                        )
                    });

            if expression_has_decorator {
                true
            } else if let Expression::Declaration(declaration_id) =
                f.context().tree.get(expression_id)
            {
                f.context()
                    .annotation_ids(*declaration_id)
                    .iter()
                    .any(|annotation_id| {
                        matches!(
                            f.context().annotation(*annotation_id),
                            Annotation::Decorator {
                                position: AnnotationPosition::BlockPrefix
                                    | AnnotationPosition::LinePrefix,
                                ..
                            }
                        )
                    })
            } else {
                false
            }
        };
        let is_in_assignment_value_context = {
            let mut current_id = node_id;

            loop {
                let Some((parent_id, parent_type)) = f.context().parent(current_id) else {
                    break false;
                };

                match parent_type {
                    NodeType::Expression => {
                        let parent_id = LocalNodeId::<Expression>::new(parent_id);
                        let parent_expression = f.context().tree.get(parent_id);
                        if matches!(
                            parent_expression,
                            Expression::Assign { right, .. } if *right == current_id
                        ) {
                            break true;
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
                                destack_ast::IfCondition::Expression { condition }
                                    if *condition == current_id
                            );
                            let is_ternary_branch = *then_expression == current_id
                                || else_expression
                                    .is_some_and(|else_expression| else_expression == current_id);

                            if is_ternary_branch || !is_ternary_test {
                                break false;
                            }
                        }

                        current_id = parent_id;
                    }
                    NodeType::Declarator => break true,
                    _ => break false,
                }
            }
        };
        let inner_has_effective_prefix_annotation = {
            let mut current_id = expression_id;

            loop {
                if f.context().has_prefix_annotation(current_id) {
                    break true;
                }

                let next_id = match f.context().tree.get(current_id) {
                    Expression::Statement(expression)
                    | Expression::Parenthesized { expression } => Some(*expression),
                    Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => {
                        Some(*left)
                    }
                    _ => None,
                };
                let Some(next_id) = next_id else {
                    break false;
                };

                current_id = next_id;
            }
        };
        let prefers_inline_scalar_comment_wrapper =
            matches!(inner_expression, Expression::ScalarLiteral(_))
                && has_parenthesized_leading_inner_comments
                && !has_parenthesized_leading_inner_newline
                && !f.context().node_has_newline(expression_id);
        let should_expand_assignment_target =
            parenthesized_assignment_target_prefers_expanded_layout(f.context(), expression_id);

        // destructuring targets
        if should_expand_assignment_target {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    group(expression).should_expand(true),
                    token(")")
                ])
                .should_expand(true)]
            )?;
        }
        // tree literal
        else if let Expression::TreeExpression {
            arguments,
            elements,
            ..
        } = inner_expression
        {
            format_parenthesized_tree_expression(
                f,
                node_id,
                expression_id,
                arguments,
                elements,
                has_parenthesized_leading_inner_trivia,
            )?;
        }
        // cast prefix comments
        else if !is_in_assignment_value_context
            && should_hoist_parenthesized_inner_cast_prefix_comments(
                f.context(),
                node_id,
                expression_id,
            )
        {
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
                    [crate::format::annotation::infix_or_postfix_annotations(
                        f.context(),
                        expression_id
                    )]
                )?;
                Ok(())
            });
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    expression_id
                )]
            )?;
            write!(
                f,
                [group(&format_args![
                    token("("),
                    format_inner_without_prefix,
                    token(")")
                ])]
            )?;
        }
        // decorator or assignment trivia
        else if has_inner_decorator_prefix_annotation
            || (is_in_assignment_value_context && has_parenthesized_leading_inner_trivia)
        {
            write!(
                f,
                [
                    token("("),
                    block_indent(&group(expression).should_expand(true)),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        }
        // type grouping and conditional types
        else if parenthesized_type_expression_prefers_soft_block_layout(
            f.context(),
            node_id,
            expression_id,
            has_parenthesized_leading_inner_comments,
            inner_has_effective_prefix_annotation,
        ) {
            write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
        }
        // leading trivia
        else if has_parenthesized_leading_inner_trivia && !prefers_inline_scalar_comment_wrapper {
            let leading_inner_comments =
                parenthesized_leading_inner_comments(f.context(), node_id, expression_id);
            let format_inner = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                for (comment_index, comment_id) in leading_inner_comments.iter().enumerate() {
                    if comment_index > 0 {
                        write!(f, [hard_line_break()])?;
                    }

                    write!(f, [*comment_id, hard_line_break()])?;
                }

                write!(f, [group(expression).should_expand(true)])
            });
            write!(
                f,
                [
                    token("("),
                    block_indent(&format_inner),
                    hard_line_break(),
                    token(")")
                ]
            )?;
        }
        // canonical fallback
        else {
            write!(f, [token("("), expression, token(")")])?;
        }

        let boundary_comments =
            parenthesized_boundary_comments(f.context(), node_id, expression_id);
        for comment_id in boundary_comments {
            write!(f, [space(), comment_id])?;
        }
    }

    Ok(())
}
