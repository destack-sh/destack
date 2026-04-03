use super::control::adjacent_statement_argument_has_leading_comments;
use crate::format::chain::transparent_inner_expression;
use crate::format::tree::tree_argument_is_wrapped_in_braces;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Expression, IfCondition, IfKind, LocalNodeId, NodeTree, NodeType, TypeLiteral,
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
        || else_expression
            .is_some_and(|expression_id| expression_has_line_comment(context, expression_id))
    {
        return true;
    }

    else_expression
        .is_some_and(|expression_id| ternary_chain_has_line_comment(context, expression_id))
}

/// Return whether one ternary chain has parenthesized then or else branches.
fn ternary_chain_has_parenthesized_branch(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((_, then_expression, else_expression)) = ternary_parts(context.tree, node_id) else {
        return false;
    };

    if matches!(
        context.tree.get(then_expression),
        Expression::Parenthesized { .. }
    ) {
        return true;
    }

    let Some(else_expression) = else_expression else {
        return false;
    };

    if matches!(
        context.tree.get(else_expression),
        Expression::Parenthesized { .. }
    ) {
        return true;
    }

    ternary_parts(context.tree, else_expression)
        .is_some_and(|_| ternary_chain_has_parenthesized_branch(context, else_expression))
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

/// Return whether one tree-like ternary branch should render without extra wrapping.
fn tree_like_branch_prefers_no_wrap(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    ternary_is_in_braced_tree_child_argument: bool,
) -> bool {
    if !ternary_branch_is_tree_like(context, expression_id) {
        return false;
    }

    if !ternary_is_in_braced_tree_child_argument {
        return false;
    }

    let expression_id = transparent_inner_expression(context, expression_id);
    let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = context.tree.get(expression_id)
    else {
        return false;
    };

    let has_arguments = arguments
        .as_ref()
        .is_some_and(|arguments| !arguments.is_empty());
    let has_elements = elements
        .as_ref()
        .is_some_and(|elements| !elements.is_empty());

    has_arguments || has_elements
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
        Expression::TypeLiteral(TypeLiteral::Null | TypeLiteral::Undefined)
    )
}

/// Return one jsx-chain branch expression id without redundant parenthesized wrappers.
fn jsx_chain_branch_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    transparent_inner_expression(context, expression_id)
}

/// Format one branch in a jsx ternary chain.
fn format_jsx_chain_branch<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
    ternary_is_in_braced_tree_child_argument: bool,
) -> FormatResult<()> {
    let wrapped_branch_expression_id = jsx_chain_branch_expression(f.context(), expression_id);

    let no_wrap = expression_is_nullish_literal(f.context(), wrapped_branch_expression_id)
        || (is_alternate && expression_is_ternary(f.context(), wrapped_branch_expression_id))
        || tree_like_branch_prefers_no_wrap(
            f.context(),
            wrapped_branch_expression_id,
            ternary_is_in_braced_tree_child_argument,
        );

    if no_wrap {
        write!(f, [expression_id])?;
        return Ok(());
    }

    write!(
        f,
        [
            if_group_breaks(&token("(")),
            soft_block_indent(&wrapped_branch_expression_id),
            if_group_breaks(&token(")"))
        ]
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
        return Ok(());
    }

    let format_then_expression = format_with(|f| {
        write!(f, [then_expression])?;
        Ok(())
    });

    let format_else_expression = format_with(|f| {
        if let Some(else_expression) = else_expression {
            write!(f, [else_expression])?;
        }
        Ok(())
    });

    let is_nested_alternate = ternary_is_nested_alternate(f.context(), node_id);
    let should_expand = force_expand
        || ternary_chain_has_line_comment(f.context(), node_id)
        || adjacent_statement_argument_has_leading_comments(f.context(), node_id)
        || adjacent_statement_argument_has_leading_comments(f.context(), condition);

    let format_inner = format_with(|f| {
        write!(f, [condition])?;
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

/// Format one ternary expression in its inline single-line form.
pub(crate) fn format_inline_ternary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    format_standard_ternary(f, node_id, true, false)
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
        || f.context().node_has_newline(node_id)
        || ternary_chain_has_parenthesized_branch(f.context(), node_id);
    let ternary_is_in_braced_tree_child_argument =
        expression_is_in_braced_tree_child_argument(f.context(), node_id);
    write!(
        f,
        [group(&format_with(|f| {
            write!(f, [condition, space(), token("?"), space()])?;
            format_jsx_chain_branch(
                f,
                then_expression,
                false,
                ternary_is_in_braced_tree_child_argument,
            )?;
            write!(f, [space(), token(":"), space()])?;
            if let Some(else_expression) = else_expression {
                format_jsx_chain_branch(
                    f,
                    else_expression,
                    true,
                    ternary_is_in_braced_tree_child_argument,
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

/// Format a ternary expression with Prettier-style breaking.
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
