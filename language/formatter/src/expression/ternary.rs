use super::conditional::ConditionalLayout;
use super::dispatch::write_expression_without_trailing_comments;
use super::parentheses::expression_is_in_template_literal_interpolation;
use crate::annotation::FormatTrailingComments;
use crate::chain::{expression_trivia_anchor_end, transparent_inner_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, Expression, IfCondition, IfKind, LocalNodeId, NodeTree, NodeType,
    ScalarLiteral,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    align, dedent, format_with, group, if_group_breaks, if_group_fits_on_line, indent,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};

const UNDEFINED_IDENTIFIER: &str = "undefined";

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

/// Return the separator comments that belong to one ternary branch boundary.
fn ternary_separator_comments<'a>(
    context: &'a DestackFormatContext<'a>,
    mut start: u32,
    end: u32,
    operator: u8,
) -> &'a [Comment] {
    let comments = context.comments().unprinted_comments();
    if comments.is_empty() {
        return &[];
    }

    let source = context.source_text();
    let mut index_before_operator = None;

    for (index, comment) in comments.iter().enumerate() {
        // stop once the next comment belongs to a later range
        if comment.span.end > end {
            return &comments[..index_before_operator.unwrap_or(index)];
        }

        // stop once the separator gap contains a newline
        if source.contains_newline_between(start, comment.span.start) {
            return &comments[..index];
        }

        // keep line comments and end-of-line comments on the left side
        if comment.is_line() || comment.followed_by_newline() {
            return &comments[..=index];
        }

        // remember the last comment that still sits before the separator token
        if source.bytes_contain(start, comment.span.start, operator) {
            index_before_operator = Some(index);
        }

        start = comment.span.end;
    }

    &comments[..index_before_operator.unwrap_or(comments.len())]
}

/// Write the separator comments that belong to one ternary branch boundary.
fn write_ternary_separator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    start: u32,
    end: u32,
    operator: u8,
) -> FormatResult<()> {
    let comments = ternary_separator_comments(f.context(), start, end, operator).to_vec();
    if comments.is_empty() {
        return Ok(());
    }

    write!(f, [FormatTrailingComments::Comments(&comments)])?;

    Ok(())
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

/// Return whether one JSX-chain branch expression can stay unwrapped.
fn expression_is_jsx_chain_bare_branch(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    let expression = context.tree.get(expression_id);

    match expression {
        Expression::ScalarLiteral(ScalarLiteral::Null) => true,
        Expression::Identifier { name } => context.strings.get(*name) == UNDEFINED_IDENTIFIER,
        _ if is_alternate => expression_is_ternary(context, expression_id),
        _ => false,
    }
}

/// Format one branch in a jsx ternary chain.
fn format_jsx_chain_branch<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
) -> FormatResult<()> {
    let branch_expression_id = transparent_inner_expression(f.context(), expression_id);
    let no_wrap =
        expression_is_jsx_chain_bare_branch(f.context(), branch_expression_id, is_alternate);

    let write_branch_body =
        format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [expression_id]));

    if no_wrap {
        return write!(f, [write_branch_body]);
    }

    write!(
        f,
        [
            if_group_breaks(&token("(")),
            soft_block_indent(&write_branch_body),
            if_group_breaks(&token(")"))
        ]
    )
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
    then_expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let format_test = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let condition_end = expression_trivia_anchor_end(f.context(), condition);
        let then_start = f.context().expression_token_start(then_expression);

        write_expression_without_trailing_comments(f, condition)?;
        write_ternary_separator_comments(f, condition_end, then_start, b'?')
    });

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
    let format_then_expression = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let then_end = expression_trivia_anchor_end(f.context(), then_expression);

        write_expression_without_trailing_comments(f, then_expression)?;

        if let Some(else_expression) = else_expression {
            let else_start = f.context().expression_token_start(else_expression);
            write_ternary_separator_comments(f, then_end, else_start, b':')?;
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
            write_expression_without_trailing_comments(f, else_expression)?;
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
        write_standard_ternary_test(f, layout, condition, then_expression)?;

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

    let layout = ternary_layout(f.context(), node_id);
    let format_inner = format_with(|f| {
        write_standard_ternary_test(f, layout, condition, then_expression)?;
        write!(f, [space(), token("?"), space()])?;
        format_jsx_chain_branch(f, then_expression, false)?;

        if let Some(else_expression) = else_expression
            && !ternary_branch_is_tree_like(f.context(), then_expression)
        {
            let then_end = expression_trivia_anchor_end(f.context(), then_expression);
            let else_start = f.context().expression_token_start(else_expression);
            write_ternary_separator_comments(f, then_end, else_start, b':')?;
        }

        write!(f, [space(), token(":"), space()])?;
        if let Some(else_expression) = else_expression {
            format_jsx_chain_branch(f, else_expression, true)?;
        }

        Ok(())
    });

    if layout.groups_at_root() {
        write!(f, [group(&format_inner)])?;
    } else {
        write!(f, [format_inner])?;
    }

    Ok(())
}

/// Return whether a template interpolation has source line breaks around the ternary body.
fn template_interpolation_has_surrounding_newline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !expression_is_in_template_literal_interpolation(context, node_id) {
        return false;
    }

    if context.is_at_line_start(node_id.id) {
        return true;
    }

    context.span_has_newline_before_next_non_whitespace_token(context.span(node_id))
}

/// Format a ternary expression with the standard breaking layout.
/// Nested ternaries get progressive indentation when they break.
pub(crate) fn format_ternary(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let keep_inline_template_ternary = ternary_parts(f.context().tree, node_id).is_some()
        && expression_is_in_template_literal_interpolation(f.context(), node_id)
        && !f.context().node_has_newline(node_id)
        && !template_interpolation_has_surrounding_newline(f.context(), node_id);

    if ternary_chain_has_tree_branch(f.context(), node_id) && !keep_inline_template_ternary {
        format_jsx_chain_ternary(f, node_id)?;
    } else {
        format_standard_ternary(f, node_id, keep_inline_template_ternary, false)?;
    }

    Ok(())
}
