use super::child::{
    expression_chain_has_separator_comment, node_has_line_comment,
    tree_child_has_outer_line_comment, tree_child_should_inline_braced_expression,
    tree_control_child_should_expand, tree_expression_contains_callback_break,
};
use crate::annotation::{
    FormatTrailingComments, format_trailing_comments, infix_or_postfix_annotations,
    prefix_annotations, prefix_annotations_after_offset, prefix_annotations_before_offset,
    write_comment_sequence,
};
use crate::chain::transparent_inner_expression;
use crate::collection::literal::format_scalar_literal;
use crate::context::with_expanded_tree_callback_bodies;
use crate::expression::{argument_value, tree_chain_ternary_needs_expanded_branches};
use crate::file::write_source_span;
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use tspp_core::ensure_sufficient_stack;
use tspp_dir::{
    Argument, Expression, IfForm, Literal, LocalNodeId, Node, NodeType, Tree, TreeAttribute,
    TreeAttributeValue, TreeChild, TreeStore,
};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_suffix_boundary, soft_block_indent,
    text, token,
};
use tspp_fir::{format_args, write};
use tspp_source::Span;

/// Return whether tree argument formatting should force multiline mode.
pub(crate) fn has_multiline_tree_argument(
    context: &TsppFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    arguments.iter().copied().any(|argument_id| {
        let Some(value_id) = argument_value(context.tree, argument_id) else {
            return false;
        };
        let value_id = transparent_inner_expression(context, value_id);

        let Expression::TreeExpression { children, .. } = context.tree.get(value_id) else {
            return false;
        };

        children
            .as_ref()
            .is_some_and(|children| !children.is_empty())
    })
}

/// Return the enclosing span used for one tree node's trailing comments.
fn tree_node_enclosing_span(context: &TsppFormatContext<'_>, node_id: u32) -> FormatResult<Span> {
    let Some((parent_id, NodeType::Expression)) = context.parent_by_id(node_id) else {
        return Err(FormatError::SyntaxError {
            message: "tree node requires one expression parent",
        });
    };

    Ok(context.span(LocalNodeId::<Expression>::new(parent_id)))
}

/// Write one empty tree expression container.
fn write_empty_tree_child<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
) -> FormatResult<()> {
    let child_span = f.context().span(child_id);
    let comments = f
        .context()
        .comments()
        .comments_in_range(child_span.start, child_span.end)
        .to_vec();

    // line comments require a multiline expression container
    if comments.iter().copied().any(|comment| comment.is_line()) {
        write!(
            f,
            [group(&format_args![
                token("{"),
                block_indent(&format_with(|f| { write_comment_sequence(f, &comments) })),
                hard_line_break(),
                token("}")
            ])]
        )?;
        return Ok(());
    }

    // block comments remain inline inside the braces
    write!(f, [token("{")])?;
    write_comment_sequence(f, &comments)?;
    write!(f, [token("}")])?;

    Ok(())
}

/// Write one tree expression container child.
fn write_tree_expression_child<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let child_span = f.context().span(child_id);
    let value_end = f.context().tree.get_source_extent(value).end;
    let has_callback_break = tree_expression_contains_callback_break(f.context(), value);

    let trailing_comments = |f: &TsppFormatter<'ast, '_>| {
        f.context()
            .comments()
            .comments_before(child_span.end)
            .iter()
            .copied()
            .filter(|comment| comment.span.start >= value_end)
            .collect::<Vec<_>>()
    };
    let force_multiline_braced_expression =
        tree_child_has_outer_line_comment(f.context(), child_id, value);

    // preserve outer line comments
    if force_multiline_braced_expression {
        write!(
            f,
            [group(&format_args![
                token("{"),
                prefix_annotations(f.context(), child_id),
                block_indent(&format_with(|f| {
                    write_tree_expression_value(f, value, true, has_callback_break)?;
                    let trailing_comments = trailing_comments(f);
                    write!(f, [FormatTrailingComments::Comments(&trailing_comments)])
                })),
                hard_line_break(),
                token("}")
            ])]
        )?;
    } else if !has_callback_break
        && tree_child_should_inline_braced_expression(f.context(), child_id)
    {
        // keep short expression containers flat
        write!(
            f,
            [
                token("{"),
                prefix_annotations(f.context(), child_id),
                value,
                FormatTrailingComments::Comments(&trailing_comments(f)),
                token("}")
            ]
        )?;
    } else if has_callback_break
        || expression_chain_has_separator_comment(f.context(), value)
        || tree_control_child_should_expand(f.context(), value)
        || matches!(
            f.context().tree.get(value),
            Expression::If {
                form: IfForm::Ternary,
                ..
            } if tree_chain_ternary_needs_expanded_branches(f.context(), value)
        )
        || matches!(
            f.context().tree.get(value),
            Expression::If {
                form: IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
                ..
            } if {
                let condition_has_line_comment = condition
                    .as_expression()
                    .is_some_and(|condition| node_has_line_comment(f.context(), condition));

                condition_has_line_comment
                    || node_has_line_comment(f.context(), *then_expression)
                    || else_expression.is_some_and(|expression_id| {
                        node_has_line_comment(f.context(), expression_id)
                    })
            }
        )
    {
        // break the value without forcing its expression container
        write!(
            f,
            [group(&format_args![
                token("{"),
                prefix_annotations(f.context(), child_id),
                format_with(|f| write_tree_expression_value(f, value, true, has_callback_break)),
                FormatTrailingComments::Comments(&trailing_comments(f)),
                token("}")
            ])]
        )?;
    } else {
        // keep simple expression containers soft
        write!(
            f,
            [group(&format_args![
                token("{"),
                soft_block_indent(&format_with(|f| {
                    write!(f, [prefix_annotations(f.context(), child_id)])?;
                    write!(f, [value])?;
                    let trailing_comments = trailing_comments(f);
                    write!(f, [FormatTrailingComments::Comments(&trailing_comments)])
                })),
                token("}")
            ])]
        )?;
    }

    Ok(())
}

/// Write one tree expression container value.
fn write_tree_expression_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    should_expand: bool,
    should_expand_tree_callback_bodies: bool,
) -> FormatResult<()> {
    let value = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(f, [group(&value_id).should_expand(should_expand)])
    });

    if should_expand_tree_callback_bodies {
        with_expanded_tree_callback_bodies(f, |f| write!(f, [value]))
    } else {
        write!(f, [value])
    }
}

/// Write one tree spread expression container.
fn write_tree_spread<'ast, T>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    let node_span = f.context().span(node_id);
    let node_start = f.context().node_token_start(node_id);
    let value_span = f.context().span::<Expression>(value);
    write!(
        f,
        [prefix_annotations_before_offset(
            f.context(),
            node_id,
            node_start
        )]
    )?;

    let has_spread_comment = !f
        .context()
        .comments()
        .comments_before(value_span.start)
        .is_empty()
        || !f
            .context()
            .comments()
            .comments_in_range(value_span.end, node_span.end)
            .is_empty();
    let spread_inner = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(
            f,
            [prefix_annotations_after_offset(
                f.context(),
                node_id,
                node_start
            )]
        )?;
        write!(f, [token("..."), value])
    });

    if has_spread_comment {
        write!(
            f,
            [group(&format_args![
                token("{"),
                soft_block_indent(&spread_inner),
                line_suffix_boundary(),
                token("}")
            ])]
        )?;
    } else {
        write!(
            f,
            [token("{"), spread_inner, line_suffix_boundary(), token("}")]
        )?;
    }

    Ok(())
}

/// Write one tree attribute value.
fn write_tree_attribute_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    value: &TreeAttributeValue,
) -> FormatResult<()> {
    match value {
        TreeAttributeValue::String(string_id) => {
            let span = Span::empty(f.context().file.id);
            write!(f, [token("=")])?;
            format_scalar_literal(&Literal::String(*string_id), span, f)
        }
        TreeAttributeValue::Expression(value_id) => {
            let has_callback_break =
                tree_expression_contains_callback_break(f.context(), *value_id);

            write!(f, [token("="), token("{")])?;
            write_tree_expression_value(f, *value_id, has_callback_break, has_callback_break)?;
            write!(f, [token("}")])
        }
    }
}

/// Write one tree attribute.
pub(crate) fn write_tree_attribute<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    attribute_id: LocalNodeId<TreeAttribute>,
    following_span_start: Option<u32>,
) -> FormatResult<()> {
    match f.context().tree.get(attribute_id) {
        TreeAttribute::Named { name, value } => {
            write!(f, [prefix_annotations(f.context(), attribute_id)])?;
            write!(f, [name])?;
            if let Some(value) = value {
                write_tree_attribute_value(f, value)?;
            }
        }
        TreeAttribute::Spread { value } => {
            write_tree_spread(f, attribute_id, *value)?;
        }
        TreeAttribute::Error => {
            write!(f, [prefix_annotations(f.context(), attribute_id)])?;
            write_source_span(f, f.context().span(attribute_id))?;
        }
    }

    if let Some(following_span_start) = following_span_start {
        let enclosing_span = tree_node_enclosing_span(f.context(), attribute_id.id)?;
        let trailing_span = f.context().span(attribute_id);
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                Some(following_span_start),
            )]
        )?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), attribute_id)])
}

/// Write one tree child.
pub(crate) fn write_tree_child<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    following_span_start: Option<u32>,
) -> FormatResult<()> {
    ensure_sufficient_stack(|| write_tree_child_inner(f, child_id, following_span_start))
}

/// Write one tree child.
fn write_tree_child_inner<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    following_span_start: Option<u32>,
) -> FormatResult<()> {
    match f.context().tree.get(child_id) {
        TreeChild::Text { value } => {
            write!(f, [text(f.context().strings.get(*value))])?;
        }
        TreeChild::Empty => {
            write_empty_tree_child(f, child_id)?;
        }
        TreeChild::Expression { value } => {
            write_tree_expression_child(f, child_id, *value)?;
        }
        TreeChild::Spread { value } => {
            write_tree_spread(f, child_id, *value)?;
        }
        TreeChild::Tree { value } => {
            write!(f, [prefix_annotations(f.context(), child_id)])?;
            write!(f, [*value])?;
        }
        TreeChild::Error => {
            write!(f, [prefix_annotations(f.context(), child_id)])?;
            write_source_span(f, f.context().span(child_id))?;
        }
    }

    if let Some(following_span_start) = following_span_start {
        let enclosing_span = tree_node_enclosing_span(f.context(), child_id.id)?;
        let trailing_span = f.context().span(child_id);
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                Some(following_span_start),
            )]
        )?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), child_id)])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, TreeAttribute> for TreeAttribute {
    fn format_node(
        &self,
        node_id: LocalNodeId<TreeAttribute>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _ = self;

        write_tree_attribute(f, node_id, None)
    }
}

impl<'ast> FormatNode<'ast, TreeChild> for TreeChild {
    fn format_node(
        &self,
        node_id: LocalNodeId<TreeChild>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _ = self;

        write_tree_child(f, node_id, None)
    }
}
