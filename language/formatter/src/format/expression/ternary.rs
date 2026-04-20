use super::conditional::ConditionalLayout;
use crate::format::annotation::{FormatLeadingComments, FormatTrailingComments};
use crate::format::chain::transparent_inner_expression;
use crate::format::tree::tree_argument_is_wrapped_in_braces;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, Expression, IfCondition, IfKind, LocalNodeId, NodeTree, NodeType,
    TypeExpression, TypeLiteral,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    align, dedent, format_with, group, if_group_breaks, if_group_fits_on_line, indent,
    soft_block_indent, soft_line_break_or_space, space, token,
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

/// Return separator comments before one ternary branch.
fn ternary_separator_comments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    context.comments_after_previous_non_trivia_token_for(expression_id)
}

/// Return comments that stay on the then branch side of the ternary separator.
fn ternary_then_separator_comments(
    context: &DestackFormatContext<'_>,
    then_expression: LocalNodeId<Expression>,
    else_expression: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let then_span = context.span(then_expression);
    let separator_start = context
        .previous_non_trivia_token_before_span(context.span(else_expression))
        .map_or(context.span(else_expression).start, |token| {
            token.span.start
        });

    let mut separator_comments = context
        .comments()
        .comments_in_range(then_span.end, separator_start)
        .to_vec();
    separator_comments.extend(
        ternary_separator_comments(context, else_expression)
            .into_iter()
            .filter(|comment| comment.is_line()),
    );
    separator_comments
}

/// Return comments that stay on the else branch side of the ternary separator.
fn ternary_else_separator_comments(
    context: &DestackFormatContext<'_>,
    else_expression: LocalNodeId<Expression>,
) -> Vec<Comment> {
    ternary_separator_comments(context, else_expression)
        .into_iter()
        .filter(|comment| comment.is_block())
        .collect()
}

/// Return whether the separator between two ternary parts has a line comment.
fn ternary_separator_has_line_comment(
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
        || ternary_separator_has_line_comment(context, condition_expression, then_expression)
        || else_expression.is_some_and(|expression_id| {
            expression_has_line_comment(context, expression_id)
                || ternary_separator_has_line_comment(context, then_expression, expression_id)
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
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };

    if parent_type == NodeType::Argument {
        let argument_id = LocalNodeId::<Argument>::new(parent_id);
        if !tree_argument_is_wrapped_in_braces(context, argument_id) {
            return false;
        }

        let Some((argument_parent_id, argument_parent_type)) = context.parent(argument_id) else {
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

    false
}

/// Return the outermost explicit wrapper that still belongs to one ternary.
fn outermost_parenthesized_ternary_wrapper(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    loop {
        let Some((parent_id, parent_type)) = context.parent(node_id) else {
            return node_id;
        };
        if parent_type != NodeType::Expression {
            return node_id;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
            return node_id;
        };
        if *expression != node_id {
            return node_id;
        }

        node_id = parent_id;
    }
}

/// Return the layout position for one ternary expression.
fn ternary_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> ConditionalLayout {
    let wrapped_id = outermost_parenthesized_ternary_wrapper(context, node_id);
    let Some((parent_id, parent_type)) = context.parent(wrapped_id) else {
        return ConditionalLayout::Root;
    };
    if parent_type != NodeType::Expression {
        return ConditionalLayout::Root;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);

    let Some((condition_id, then_expression_id, else_expression_id)) =
        ternary_parts(context.tree, parent_expression_id)
    else {
        return ConditionalLayout::Root;
    };

    if condition_id.id == wrapped_id.id {
        return ConditionalLayout::NestedTest;
    }

    if then_expression_id.id == wrapped_id.id {
        return ConditionalLayout::NestedConsequent;
    }

    if else_expression_id.is_some_and(|else_expression_id| else_expression_id.id == wrapped_id.id) {
        return ConditionalLayout::NestedAlternate;
    }

    ConditionalLayout::Root
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
        || tree_expression_stays_unwrapped;

    // inline branches
    let write_branch_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !leading_comment_nodes.is_empty() {
            write!(f, [FormatLeadingComments::Comments(leading_comment_nodes)])?;
        }

        write!(f, [expression_id])?;

        if !inner_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [FormatTrailingComments::Comments(
                    inner_trailing_comment_nodes
                )]
            )?;
        }

        Ok(())
    });

    if no_wrap {
        write!(f, [write_branch_body])?;

        if !outer_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [FormatTrailingComments::Comments(
                    outer_trailing_comment_nodes
                )]
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
            [FormatTrailingComments::Comments(
                outer_trailing_comment_nodes
            )]
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

/// Return whether a ternary branch needs inline disambiguating parentheses.
fn ternary_branch_needs_inline_parentheses(
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

/// Write one ternary test using the current layout.
fn write_standard_ternary_test<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    layout: ConditionalLayout,
    condition: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let format_test = format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [condition]));

    if layout.is_nested_alternate() {
        write!(f, [align(2, &format_test)])?;
    } else {
        write!(f, [format_test])?;
    }

    Ok(())
}

/// Write one standard ternary tail.
fn write_standard_ternary_tail<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _layout: ConditionalLayout,
    then_expression: LocalNodeId<Expression>,
    else_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let then_leading_comment_nodes = ternary_separator_comments(f.context(), then_expression);
    let then_trailing_comment_nodes = else_expression
        .map(|else_expression| {
            ternary_then_separator_comments(f.context(), then_expression, else_expression)
        })
        .unwrap_or_default();
    let else_leading_comment_nodes = else_expression
        .map(|else_expression| ternary_else_separator_comments(f.context(), else_expression))
        .unwrap_or_default();

    let format_then_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !then_leading_comment_nodes.is_empty() {
            write!(
                f,
                [FormatLeadingComments::Comments(&then_leading_comment_nodes)]
            )?;
        }

        write!(f, [then_expression])?;

        if !then_trailing_comment_nodes.is_empty() {
            write!(
                f,
                [FormatTrailingComments::Comments(
                    &then_trailing_comment_nodes
                )]
            )?;
        }

        Ok(())
    });

    let format_then_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let format_then_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if f.options().indent_style.is_space() {
                write!(f, [align(2, &format_then_expression)])?;
            } else {
                write!(f, [indent(&format_then_expression)])?;
            }

            Ok(())
        });

        if ternary_branch_needs_inline_parentheses(f.context(), then_expression) {
            write!(
                f,
                [
                    if_group_fits_on_line(&token("(")),
                    format_then_expression,
                    if_group_fits_on_line(&token(")"))
                ]
            )?;
        } else {
            write!(f, [format_then_expression])?;
        }

        Ok(())
    });

    let format_else_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if let Some(else_expression) = else_expression {
            if !else_leading_comment_nodes.is_empty() {
                write!(
                    f,
                    [FormatLeadingComments::Comments(&else_leading_comment_nodes)]
                )?;
            }

            write!(f, [else_expression])?;
        }
        Ok(())
    });

    let format_else_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.options().indent_style.is_space() {
            write!(f, [align(2, &format_else_expression)])?;
        } else {
            write!(f, [indent(&format_else_expression)])?;
        }

        Ok(())
    });

    write!(
        f,
        [format_args![
            soft_line_break_or_space(),
            token("?"),
            space(),
            format_then_expression,
            soft_line_break_or_space(),
            token(":"),
            space(),
            format_else_expression
        ]]
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

    let layout = ternary_layout(f.context(), node_id);
    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_standard_ternary_test(f, layout, condition)?;

        let format_tail = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_standard_ternary_tail(f, layout, then_expression, else_expression)
        });

        match layout {
            ConditionalLayout::Root | ConditionalLayout::NestedTest => {
                write!(f, [indent(&format_tail)])?;
            }
            ConditionalLayout::NestedConsequent => {
                write!(f, [dedent(&indent(&format_tail))])?;
            }
            ConditionalLayout::NestedAlternate => {
                write!(f, [format_tail])?;
            }
        }

        Ok(())
    });

    let grouped = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if layout.groups_at_root() {
            let grouped = group(&format_inner).should_expand(force_expand);
            write!(f, [grouped])?;
        } else {
            write!(f, [format_inner])?;
        }

        Ok(())
    });

    if layout.is_nested_test() {
        write!(f, [group(&soft_block_indent(&grouped))])?;
    } else {
        write!(f, [grouped])?;
    }

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

    // separator comments before the alternate belong to the alternate wrapper
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
            let mut inner_own_line_trailing_comment_nodes = Vec::new();
            let mut outer_inline_trailing_comment_nodes = Vec::new();

            // own-line trailing comments stay inside the alternate wrapper,
            // while inline trailing comments stay outside it
            for comment in inner_trailing_comment_nodes {
                if f.context().span_starts_on_own_line(comment.span) {
                    inner_own_line_trailing_comment_nodes.push(comment);
                }
            }

            for comment in outer_trailing_comment_nodes {
                if !f.context().span_starts_on_own_line(comment.span) {
                    outer_inline_trailing_comment_nodes.push(comment);
                }
            }

            (
                inner_own_line_trailing_comment_nodes,
                outer_inline_trailing_comment_nodes,
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
fn template_interpolation_has_surrounding_newline(
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
        && !template_interpolation_has_surrounding_newline(f.context(), node_id);

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
