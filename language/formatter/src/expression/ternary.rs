use super::conditional::ConditionalLayout;
use super::dispatch::write_expression_without_trailing_comments;
use crate::annotation::{FormatTrailingComments, write_comment_slice};
use crate::chain::{expression_trivia_anchor_end, transparent_inner_expression};
use crate::{TsppFormatContext, TsppFormatter};
use smallvec::SmallVec;
use tspp_dir::{Argument, Comment, Expression, IfForm, Literal, LocalNodeId, NodeType, Tree};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{
    align, dedent, format_with, group, if_group_breaks, if_group_fits_on_line, indent,
    soft_block_indent, soft_line_break_or_space, space, token,
};
use tspp_fir::{format_args, write};

/// Return the value expression for an argument.
pub(crate) fn argument_value(
    tree: &Tree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Positional { value, .. } => Some(*value),
        _ => None,
    }
}

/// Return ternary components for one expression node.
fn ternary_parts(
    tree: &Tree,
    node_id: LocalNodeId<Expression>,
) -> Option<(
    LocalNodeId<Expression>,
    LocalNodeId<Expression>,
    Option<LocalNodeId<Expression>>,
)> {
    let Expression::If {
        form: IfForm::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = tree.get(node_id)
    else {
        return None;
    };

    let condition_id = condition.as_expression()?;

    Some((condition_id, *then_expression, *else_expression))
}

/// Return whether a ternary branch expression is tree-like.
pub(crate) fn ternary_branch_is_tree_like(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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

/// Return whether one tree ternary chain must expand to preserve branch ownership.
pub(crate) fn tree_chain_ternary_needs_expanded_branches(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((_, then_expression, else_expression)) = ternary_parts(context.tree, node_id) else {
        return false;
    };

    ternary_branch_has_breaking_trailing_comments(context, then_expression)
        || else_expression.is_some_and(|expression_id| {
            ternary_branch_has_breaking_trailing_comments(context, expression_id)
        })
}

/// Return comments before one ternary separator token.
fn ternary_separator_comments<'a>(
    context: &'a TsppFormatContext<'a>,
    mut start: u32,
    end: u32,
    operator: u8,
) -> &'a [Comment] {
    let comments = context.comments().unprinted_comments();
    if comments.is_empty() {
        return &[];
    }

    let source = context.source_text();

    for (index, comment) in comments.iter().enumerate() {
        // stop once the next comment belongs to a later range
        if comment.span.end > end {
            return &comments[..index];
        }

        // stop once the separator gap contains a newline
        if source.contains_newline_between(start, comment.span.start) {
            return &comments[..index];
        }

        // stop once the separator belongs to the right branch
        if source.bytes_contain(start, comment.span.start, operator) {
            return &comments[..index];
        }

        // keep line comments and end-of-line comments on the left side
        if comment.is_line() || comment.followed_by_newline() {
            return &comments[..=index];
        }

        start = comment.span.end;
    }

    comments
}

/// Write the separator comments that belong to one ternary branch boundary.
fn write_ternary_separator_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    start: u32,
    end: u32,
    operator: u8,
) -> FormatResult<()> {
    let comments: SmallVec<[Comment; 2]> =
        ternary_separator_comments(f.context(), start, end, operator)
            .iter()
            .copied()
            .collect();
    if comments.is_empty() {
        return Ok(());
    }

    write!(f, [FormatTrailingComments::Comments(comments.as_slice())])?;

    Ok(())
}

/// Return whether one expression is a ternary expression.
fn expression_is_ternary(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::If {
            form: IfForm::Ternary,
            ..
        }
    )
}

/// Return the layout position for one ternary expression.
fn ternary_layout(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> ConditionalLayout {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
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

    if condition_id.id == node_id.id {
        return ConditionalLayout::NestedTest;
    }

    if then_expression_id.id == node_id.id {
        return ConditionalLayout::NestedConsequent;
    }

    if else_expression_id.is_some_and(|else_expression_id| else_expression_id.id == node_id.id) {
        return ConditionalLayout::NestedAlternate;
    }

    ConditionalLayout::Root
}

/// Return whether one tree-chain branch expression can stay unwrapped.
fn expression_is_tree_chain_bare_branch(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    let expression = context.tree.get(expression_id);

    match expression {
        Expression::Literal(Literal::Null | Literal::Undefined) => true,
        _ if is_alternate => expression_is_ternary(context, expression_id),
        _ => false,
    }
}

/// Return the parent ternary for one branch expression.
fn ternary_branch_parent(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent_by_id(expression_id.id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let (_, then_expression, else_expression) = ternary_parts(context.tree, parent_id)?;
    if then_expression == expression_id || else_expression.is_some_and(|id| id == expression_id) {
        return Some(parent_id);
    }

    None
}

/// Return whether one expression is a direct ternary branch.
pub(crate) fn expression_is_ternary_branch(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    ternary_branch_parent(context, expression_id).is_some()
}

/// Return trailing comments attached to one ternary branch.
pub(crate) fn ternary_branch_trailing_comments<'a>(
    context: &'a TsppFormatContext<'a>,
    expression_id: LocalNodeId<Expression>,
) -> Option<(u32, &'a [Comment])> {
    let branch_end = expression_trivia_anchor_end(context, expression_id);
    let parent_id = ternary_branch_parent(context, expression_id)?;

    let Some((_, then_expression, else_expression)) = ternary_parts(context.tree, parent_id) else {
        return Some((branch_end, &[]));
    };

    if then_expression == expression_id {
        let Some(else_expression) = else_expression else {
            return Some((branch_end, &[]));
        };

        let else_start = context.expression_token_start(else_expression);
        let comments = ternary_separator_comments(context, branch_end, else_start, b':');

        return Some((branch_end, comments));
    }

    // cover comments owned by the branch parentheses or the complete ternary
    let parent_span = context.tree.get_source_extent(parent_id);
    let branch_span = context.tree.get_source_extent(expression_id);
    let trailing_span = parent_span.merge(branch_span);
    let comments = context
        .comments()
        .comments_in_range(branch_end, trailing_span.end);

    Some((branch_end, comments))
}

/// Return whether branch trailing comments require the branch to break.
fn ternary_branch_has_breaking_trailing_comments(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((branch_end, comments)) = ternary_branch_trailing_comments(context, expression_id)
    else {
        return false;
    };

    comments.iter().any(|comment| {
        comment.is_line()
            || context
                .source_text()
                .contains_newline_between(branch_end, comment.span.start)
    })
}

/// Format one branch in a tree ternary chain.
fn format_tree_chain_branch<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    is_alternate: bool,
) -> FormatResult<()> {
    let branch_expression_id = transparent_inner_expression(f.context(), expression_id);
    let no_wrap =
        expression_is_tree_chain_bare_branch(f.context(), branch_expression_id, is_alternate);
    let branch_comments: SmallVec<[Comment; 2]> =
        ternary_branch_trailing_comments(f.context(), expression_id)
            .map(|(_, comments)| comments)
            .unwrap_or(&[])
            .iter()
            .copied()
            .collect();

    let write_branch_body = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if branch_comments.is_empty() {
            write!(f, [expression_id])?;
        } else {
            write_expression_without_trailing_comments(f, expression_id)?;
            write_comment_slice(f, branch_comments.as_slice())?;
        }

        Ok(())
    });

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

/// Return whether a ternary branch needs inline disambiguating parentheses.
fn ternary_branch_needs_inline_parentheses(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::If {
            form: IfForm::Ternary,
            ..
        }
    )
}

/// Write one ternary test using the current layout.
fn write_standard_ternary_test<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    layout: ConditionalLayout,
    condition: LocalNodeId<Expression>,
    then_expression: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let format_test = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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
    f: &mut TsppFormatter<'ast, '_>,
    _layout: ConditionalLayout,
    then_expression: LocalNodeId<Expression>,
    else_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let format_then_expression = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let then_end = expression_trivia_anchor_end(f.context(), then_expression);

        write_expression_without_trailing_comments(f, then_expression)?;

        if let Some(else_expression) = else_expression {
            let else_start = f.context().expression_token_start(else_expression);
            write_ternary_separator_comments(f, then_end, else_start, b':')?;
        }

        Ok(())
    });

    let format_then_expression = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let format_then_expression = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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

    let format_else_expression = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if let Some(else_expression) = else_expression {
            write_expression_without_trailing_comments(f, else_expression)?;
        }
        Ok(())
    });

    let format_else_expression = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    force_expand: bool,
) -> FormatResult<()> {
    let Some((condition, then_expression, else_expression)) =
        ternary_parts(f.context().tree, node_id)
    else {
        return Ok(());
    };

    let layout = ternary_layout(f.context(), node_id);
    let format_inner = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write_standard_ternary_test(f, layout, condition, then_expression)?;

        let format_tail = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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

    let grouped = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    format_standard_ternary(f, node_id, true)
}

/// Format one tree ternary chain expression.
fn format_tree_chain_ternary<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
        format_tree_chain_branch(f, then_expression, false)?;

        write!(f, [space(), token(":"), space()])?;
        if let Some(else_expression) = else_expression {
            format_tree_chain_branch(f, else_expression, true)?;
        }

        Ok(())
    });

    if layout.groups_at_root() {
        let should_expand = tree_chain_ternary_needs_expanded_branches(f.context(), node_id);

        write!(f, [group(&format_inner).should_expand(should_expand)])?;
    } else {
        write!(f, [format_inner])?;
    }

    Ok(())
}

/// Format a ternary expression with the standard breaking layout.
/// Nested ternaries get progressive indentation when they break.
pub(crate) fn format_ternary(
    f: &mut TsppFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if ternary_chain_has_tree_branch(f.context(), node_id) {
        format_tree_chain_ternary(f, node_id)?;
    } else {
        format_standard_ternary(f, node_id, false)?;
    }

    Ok(())
}
