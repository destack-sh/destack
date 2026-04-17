use super::control::adjacent_statement_argument_has_leading_comments;
use crate::format::annotation::{format_trailing_comment_slice, write_raw_leading_comments};
use crate::format::chain::transparent_inner_expression;
use crate::format::tree::tree_argument_is_wrapped_in_braces;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, Expression, IfCondition, IfKind, LocalNodeId, NodeTree, NodeType,
    TypeExpression, TypeLiteral,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, if_group_breaks, indent, soft_block_indent, soft_line_break_or_space,
    space, token,
};
use destack_fir::{format_args, write};

/// Return the value expression for an argument.
pub(crate) fn argument_value(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Positional { value, .. } => Some(*value),
        _ => None,
    }
}

/// Return ternary components for one expression node.
fn ternary_parts(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> Option<(
    LocalNodeId<Expression>,
    LocalNodeId<Expression>,
    Option<LocalNodeId<Expression>>,
)> {
    let Expression::If {
        kind: IfKind::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = tree.get(node_id)
    else {
        return None;
    };

    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => return None,
    };

    Some((condition_id, *then_expression, *else_expression))
}

/// Return whether a ternary expression appears in statement position.
pub(crate) fn ternary_requires_terminator(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((_, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type == NodeType::Block {
        return true;
    }

    false
}

/// Return whether a ternary branch expression is tree-like.
pub(crate) fn ternary_branch_is_tree_like(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TreeExpression { .. }
    )
}

/// Return whether any branch in a ternary chain is tree-like.
fn ternary_chain_has_tree_branch(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((_, then_expression, else_expression)) = ternary_parts(context.tree, node_id) else {
        return false;
    };

    if ternary_branch_is_tree_like(context, then_expression) {
        return true;
    }

    let Some(else_expression) = else_expression else {
        return false;
    };

    if ternary_branch_is_tree_like(context, else_expression) {
        return true;
    }

    ternary_parts(context.tree, else_expression)
        .is_some_and(|_| ternary_chain_has_tree_branch(context, else_expression))
}

/// Return whether one expression has a line slash comment.
fn expression_has_line_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.span(expression_id);

    context
        .comments_in_range(expression_span.start, expression_span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return separator-owned comments before one ternary branch.
fn ternary_separator_comments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Vec<destack_ast::Comment> {
    context.raw_comments_after_previous_non_trivia_token_for(expression_id)
}

/// Return comments that stay on the then branch side of the ternary boundary.
fn ternary_then_boundary_comments(
    context: &DestackFormatContext<'_>,
    then_expression: LocalNodeId<Expression>,
    else_expression: LocalNodeId<Expression>,
) -> Vec<destack_ast::Comment> {
    let then_span = context.span(then_expression);
    let separator_start = context
        .previous_non_trivia_token_before_span(context.span(else_expression))
        .map_or(context.span(else_expression).start, |token| {
            token.span.start
        });

    let mut boundary_comments =
        context.raw_boundary_comments_in_range(then_span.end, separator_start);
    boundary_comments.extend(
        ternary_separator_comments(context, else_expression)
            .into_iter()
            .filter(|comment| comment.is_line()),
    );
    boundary_comments
}

/// Return comments that stay on the else branch side of the ternary boundary.
fn ternary_else_boundary_comments(
    context: &DestackFormatContext<'_>,
    else_expression: LocalNodeId<Expression>,
) -> Vec<destack_ast::Comment> {
    ternary_separator_comments(context, else_expression)
        .into_iter()
        .filter(|comment| comment.is_block())
        .collect()
}

/// Return whether the boundary between two ternary parts has a line comment.
fn ternary_boundary_has_line_comment(
    context: &DestackFormatContext<'_>,
    _left_expression: LocalNodeId<Expression>,
    right_expression: LocalNodeId<Expression>,
) -> bool {
    ternary_separator_comments(context, right_expression)
        .iter()
        .any(|comment| comment.is_line())
}

/// Return whether one ternary chain has line slash comments on any condition or branch.
fn ternary_chain_has_line_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((condition_expression, then_expression, else_expression)) =
        ternary_parts(context.tree, node_id)
    else {
        return false;
    };

    if expression_has_line_comment(context, condition_expression)
        || expression_has_line_comment(context, then_expression)
        || ternary_boundary_has_line_comment(context, condition_expression, then_expression)
        || else_expression.is_some_and(|expression_id| {
            expression_has_line_comment(context, expression_id)
                || ternary_boundary_has_line_comment(context, then_expression, expression_id)
        })
    {
        return true;
    }

    else_expression
        .is_some_and(|expression_id| ternary_chain_has_line_comment(context, expression_id))
}

/// Return whether one expression is a ternary expression.
fn expression_is_ternary(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    )
}

/// Return whether one expression is inside a braced tree child argument.
fn expression_is_in_braced_tree_child_argument(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = expression_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_expression_id) else {
            return false;
        };

        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);
            if !tree_argument_is_wrapped_in_braces(context, argument_id) {
                return false;
            }

            let Some((argument_parent_id, argument_parent_type)) = context.parent(argument_id)
            else {
                return false;
            };
            if argument_parent_type != NodeType::Expression {
                return false;
            }

            let tree_expression_id = LocalNodeId::<Expression>::new(argument_parent_id);
            return matches!(
                context.tree.get(tree_expression_id),
                Expression::TreeExpression { .. }
            );
        }

        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = context.tree.get(parent_expression_id);
        let Expression::Parenthesized { expression } = parent_expression else {
            return false;
        };
        if expression.id != current_expression_id.id {
            return false;
        }

        current_expression_id = parent_expression_id;
    }
}

/// Return whether one ternary is the alternate branch of a parent ternary.
fn ternary_is_nested_alternate(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind: IfKind::Ternary,
        else_expression,
        ..
    } = context.tree.get(parent_expression_id)
    else {
        return false;
    };

    else_expression.is_some_and(|else_expression| else_expression.id == node_id.id)
}

/// Return whether one expression is a nullish literal.
fn expression_is_nullish_literal(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Type {
            value
        } if matches!(
            context.tree.get(*value),
            TypeExpression::Literal {
                value: TypeLiteral::Null | TypeLiteral::Undefined
            }
        )
    )
}

/// Format one branch in a jsx ternary chain.
fn format_jsx_chain_branch<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
    ternary_is_in_braced_tree_child_argument: bool,
    leading_comment_nodes: &[Comment],
    inner_trailing_comment_nodes: &[Comment],
    outer_trailing_comment_nodes: &[Comment],
) -> FormatResult<()> {
    let branch_expression_id = transparent_inner_expression(f.context(), expression_id);
    let tree_expression_stays_unwrapped = if ternary_is_in_braced_tree_child_argument {
        match f.context().tree.get(branch_expression_id) {
            Expression::TreeExpression {
                arguments,
                elements,
                ..
            } => {
                arguments
                    .as_ref()
                    .is_some_and(|arguments| !arguments.is_empty())
                    || elements
                        .as_ref()
                        .is_some_and(|elements| !elements.is_empty())
            }
            _ => false,
        }
    } else {
        false
    };

    let no_wrap = expression_is_nullish_literal(f.context(), branch_expression_id)
        || (is_alternate && expression_is_ternary(f.context(), branch_expression_id))
        || matches!(
            f.context().tree.get(expression_id),
            Expression::Parenthesized { .. }
        )
        || tree_expression_stays_unwrapped;

    // inline branches
    let write_branch_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !leading_comment_nodes.is_empty() {
            write_raw_leading_comments(f, leading_comment_nodes)?;
        }

        write!(f, [expression_id])?;

        if !inner_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [format_trailing_comment_slice(inner_trailing_comment_nodes)]
            )?;
        }

        Ok(())
    });

    if no_wrap {
        write!(f, [write_branch_body])?;

        if !outer_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [format_trailing_comment_slice(outer_trailing_comment_nodes)]
            )?;
        }

        return Ok(());
    }

    write!(
        f,
        [
            if_group_breaks(&token("(")),
            soft_block_indent(&write_branch_body),
            if_group_breaks(&token(")"))
        ]
    )?;

    if !outer_trailing_comment_nodes.is_empty() {
        write!(
            f,
            [format_trailing_comment_slice(outer_trailing_comment_nodes)]
        )?;
    }

    Ok(())
}

/// Format one standard ternary expression.
fn write_inline_template_ternary<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    condition: LocalNodeId<Expression>,
    then_expression: LocalNodeId<Expression>,
    else_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_args![
            condition,
            space(),
            token("?"),
            space(),
            then_expression,
            space(),
            token(":"),
            space(),
            else_expression
        ])]
    )?;

    Ok(())
}

/// Return whether one standard ternary should expand.
fn standard_ternary_should_expand(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    condition: LocalNodeId<Expression>,
    force_expand: bool,
) -> bool {
    force_expand
        || ternary_chain_has_line_comment(context, node_id)
        || adjacent_statement_argument_has_leading_comments(context, node_id)
        || adjacent_statement_argument_has_leading_comments(context, condition)
}

/// Write one standard ternary tail.
fn write_standard_ternary_tail<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    then_expression: LocalNodeId<Expression>,
    else_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let then_leading_comment_nodes = ternary_separator_comments(f.context(), then_expression);
    let then_trailing_comment_nodes = else_expression
        .map(|else_expression| {
            ternary_then_boundary_comments(f.context(), then_expression, else_expression)
        })
        .unwrap_or_default();
    let else_leading_comment_nodes = else_expression
        .map(|else_expression| ternary_else_boundary_comments(f.context(), else_expression))
        .unwrap_or_default();

    let format_then_expression = format_with(|f| {
        if !then_leading_comment_nodes.is_empty() {
            write_raw_leading_comments(f, &then_leading_comment_nodes)?;
        }

        write!(f, [then_expression])?;

        if !then_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [format_trailing_comment_slice(&then_trailing_comment_nodes)]
            )?;
        }

        Ok(())
    });

    let format_else_expression = format_with(|f| {
        if let Some(else_expression) = else_expression {
            if !else_leading_comment_nodes.is_empty() {
                write_raw_leading_comments(f, &else_leading_comment_nodes)?;
            }

            write!(f, [else_expression])?;
        }
        Ok(())
    });

    write!(
        f,
        [indent(&format_args![
            soft_line_break_or_space(),
            token("?"),
            space(),
            format_then_expression,
            soft_line_break_or_space(),
            token(":"),
            space(),
            format_else_expression
        ])]
    )?;

    Ok(())
}

/// Format one standard ternary expression.
fn format_standard_ternary<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    keep_inline_template_ternary: bool,
    force_expand: bool,
) -> FormatResult<()> {
    let Some((condition, then_expression, else_expression)) =
        ternary_parts(f.context().tree, node_id)
    else {
        return Ok(());
    };

    if keep_inline_template_ternary {
        return write_inline_template_ternary(f, condition, then_expression, else_expression);
    }

    let is_nested_alternate = ternary_is_nested_alternate(f.context(), node_id);
    let should_expand =
        standard_ternary_should_expand(f.context(), node_id, condition, force_expand);

    let format_inner = format_with(|f| {
        write!(f, [condition])?;
        write_standard_ternary_tail(f, then_expression, else_expression)?;
        Ok(())
    });

    write!(
        f,
        [format_with(|f| {
            if is_nested_alternate {
                write!(f, [format_inner])?;
            } else {
                write!(f, [group(&format_inner).should_expand(should_expand)])?;
            }
            Ok(())
        })]
    )?;

    Ok(())
}

/// Format one ternary expression in its expanded multiline form.
pub(crate) fn format_expanded_ternary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    format_standard_ternary(f, node_id, false, true)
}

/// Format one jsx ternary chain expression.
fn format_jsx_chain_ternary<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let Some((condition, then_expression, else_expression)) =
        ternary_parts(f.context().tree, node_id)
    else {
        return Ok(());
    };

    let should_expand = ternary_chain_has_line_comment(f.context(), node_id)
        || f.context().node_has_newline(node_id);
    let ternary_is_in_braced_tree_child_argument =
        expression_is_in_braced_tree_child_argument(f.context(), node_id);
    let ternary_span = f.context().span(node_id);

    // boundary comments before the alternate belong to the alternate wrapper
    let alternate_leading_comment_nodes = else_expression
        .map(|else_expression| {
            let then_span = f.context().span(then_expression);
            let else_span = f.context().span(else_expression);

            let comments = f.context().comments();
            comments
                .comments_in_range(then_span.end, else_span.start)
                .to_vec()
        })
        .unwrap_or_default();
    let (alternate_inner_trailing_comment_nodes, alternate_outer_trailing_comment_nodes) =
        if let Some(else_expression) = else_expression {
            let else_span = f.context().span(else_expression);
            let inner_trailing_comment_nodes = {
                let comments = f.context().comments();
                comments
                    .comments_in_range(else_span.end, ternary_span.end)
                    .to_vec()
            };
            let outer_trailing_comment_nodes = {
                let comments = f.context().comments();
                let mut collected = Vec::new();
                let mut current = else_span.end;

                for comment in comments.comments_after(else_span.end).iter().copied() {
                    if !f.context().source_text().all_bytes_match(
                        current,
                        comment.span.start,
                        |byte| byte.is_ascii_whitespace(),
                    ) {
                        break;
                    }

                    collected.push(comment);
                    current = comment.span.end;

                    if comment.is_line() || comment.followed_by_newline() {
                        break;
                    }
                }

                collected
            };
            let mut owned_inner_trailing_comment_nodes = Vec::new();
            let mut owned_outer_trailing_comment_nodes = Vec::new();

            // own-line trailing comments stay inside the alternate wrapper,
            // while inline trailing comments stay outside it
            for comment in inner_trailing_comment_nodes {
                if f.context().span_starts_on_own_line(comment.span) {
                    owned_inner_trailing_comment_nodes.push(comment);
                }
            }

            for comment in outer_trailing_comment_nodes {
                if !f.context().span_starts_on_own_line(comment.span) {
                    owned_outer_trailing_comment_nodes.push(comment);
                }
            }

            (
                owned_inner_trailing_comment_nodes,
                owned_outer_trailing_comment_nodes,
            )
        } else {
            (Vec::new(), Vec::new())
        };

    write!(
        f,
        [group(&format_with(|f| {
            write!(f, [condition, space(), token("?"), space()])?;
            format_jsx_chain_branch(
                f,
                then_expression,
                false,
                ternary_is_in_braced_tree_child_argument,
                &[],
                &[],
                &[],
            )?;
            write!(f, [space(), token(":"), space()])?;
            if let Some(else_expression) = else_expression {
                format_jsx_chain_branch(
                    f,
                    else_expression,
                    true,
                    ternary_is_in_braced_tree_child_argument,
                    &alternate_leading_comment_nodes,
                    &alternate_inner_trailing_comment_nodes,
                    &alternate_outer_trailing_comment_nodes,
                )?;
            }
            Ok(())
        }))
        .should_expand(should_expand)]
    )?;

    Ok(())
}

/// Return whether a template interpolation has source line breaks around the ternary body.
fn template_interpolation_has_boundary_newline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !context.expression_is_in_template_literal_interpolation(node_id) {
        return false;
    }

    if context.is_at_line_start(node_id.id) {
        return true;
    }

    context.span_has_newline_before_next_non_whitespace_token(context.span(node_id))
}

/// Format a ternary expression with OXC-shaped breaking.
/// Nested ternaries get progressive indentation when they break.
pub(crate) fn format_ternary(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let keep_inline_template_ternary = ternary_parts(f.context().tree, node_id).is_some()
        && f.context()
            .expression_is_in_template_literal_interpolation(node_id)
        && !f.context().node_has_newline(node_id)
        && !template_interpolation_has_boundary_newline(f.context(), node_id);

    if ternary_chain_has_tree_branch(f.context(), node_id) && !keep_inline_template_ternary {
        format_jsx_chain_ternary(f, node_id)?;
    } else {
        format_standard_ternary(f, node_id, keep_inline_template_ternary, false)?;
    }

    // statement-position ternaries keep explicit terminators
    if ternary_requires_terminator(f.context(), node_id) {
        write!(f, [token(";")])?;
    }

    Ok(())
}
