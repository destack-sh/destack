use super::child::{
    expression_chain_has_separator_comment, format_inline_stub_comments,
    format_multiline_stub_comment_nodes, node_has_line_comment, tree_child_has_outer_line_comment,
    tree_child_should_inline_braced_expression, tree_control_child_should_expand,
    tree_expression_contains_callback_break,
};
use crate::annotation::{
    FormatTrailingComments, format_trailing_comments, infix_or_postfix_annotations,
    prefix_annotations, prefix_annotations_after_offset, prefix_annotations_before_offset,
};
use crate::chain::transparent_inner_expression;
use crate::collection::literal::format_scalar_literal;
use crate::context::with_expanded_tree_callback_bodies;
use crate::expression::{argument_value, jsx_chain_ternary_needs_expanded_branches};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_dir::{
    Argument, Comment, Expression, IfForm, LocalNodeId, NodeType, ScalarLiteral, TreeAttribute,
    TreeAttributeValue, TreeChild,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_suffix_boundary, soft_block_indent,
    text, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

/// Return whether JSX argument formatting should force multiline mode.
pub(crate) fn has_multiline_jsx_argument(
    context: &DestackFormatContext<'_>,
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
fn tree_node_enclosing_span(
    context: &DestackFormatContext<'_>,
    node_id: u32,
    node_type: NodeType,
) -> Span {
    let Some((parent_id, parent_type)) = context.parent_by_id(node_id) else {
        unreachable!("tree node should have one parent");
    };

    match parent_type {
        NodeType::Expression => context.span(LocalNodeId::<Expression>::new(parent_id)),
        _ => unreachable!("{} node parent should be one expression", node_type.name()),
    }
}

/// Return stub comment nodes attached to the value span or child span.
fn stub_child_comment_nodes(
    context: &DestackFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
    value_id: LocalNodeId<Expression>,
) -> (Vec<Comment>, bool) {
    let expression_comment_nodes = {
        let comments = context.comments();
        comments
            .comments_in_range(context.span(value_id).start, context.span(value_id).end)
            .to_vec()
    };
    if !expression_comment_nodes.is_empty() {
        return (expression_comment_nodes, false);
    }

    let child_comment_nodes = {
        let comments = context.comments();
        comments
            .comments_in_range(context.span(child_id).start, context.span(child_id).end)
            .to_vec()
    };
    let rendered_inline_from_child = !child_comment_nodes.is_empty();

    (child_comment_nodes, rendered_inline_from_child)
}

/// Write one stub tree child inside `{ ... }`.
fn write_stub_tree_child<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let (comment_nodes, rendered_inline_from_child) =
        stub_child_comment_nodes(f.context(), child_id, value_id);

    if comment_nodes
        .iter()
        .copied()
        .any(|comment| comment.is_line())
    {
        write!(
            f,
            [group(&format_args![
                token("{"),
                block_indent(&format_with(|f| {
                    format_multiline_stub_comment_nodes(f, &comment_nodes)
                })),
                hard_line_break(),
                token("}")
            ])]
        )?;
    } else {
        write!(f, [token("{")])?;
        let mut wrote_stub_comment = format_inline_stub_comments(f, f.context().span(value_id))?;
        if !wrote_stub_comment {
            wrote_stub_comment = format_inline_stub_comments(f, f.context().span(child_id))?;
        }
        write!(f, [token("}")])?;

        return Ok(rendered_inline_from_child && wrote_stub_comment);
    }

    Ok(rendered_inline_from_child)
}

/// Write one tree expression container child.
fn write_tree_expression_child<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    value: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let value_expr = f.context().tree.get(value);
    let child_span = f.context().span(child_id);
    let value_end = f.context().tree.get_source_extent(value).end;
    let has_callback_break = tree_expression_contains_callback_break(f.context(), value);

    let trailing_comments = |f: &DestackFormatter<'ast, '_>| {
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

    if matches!(value_expr, Expression::Stub) {
        return write_stub_tree_child(f, child_id, value);
    }

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
            } if jsx_chain_ternary_needs_expanded_branches(f.context(), value)
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
        // break complex expression containers
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

    Ok(false)
}

/// Write one tree expression container value.
fn write_tree_expression_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    should_expand: bool,
    should_expand_tree_callback_bodies: bool,
) -> FormatResult<()> {
    let value = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [group(&value_id).should_expand(should_expand)])
    });

    if should_expand_tree_callback_bodies {
        with_expanded_tree_callback_bodies(f, |f| write!(f, [value]))
    } else {
        write!(f, [value])
    }
}

/// Write one tree spread expression container.
fn write_tree_spread_attribute<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    attribute_id: LocalNodeId<TreeAttribute>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let node_span = f.context().span(attribute_id);
    let node_start = f.context().node_token_start(attribute_id);
    let value_span = f.context().span(value);
    write!(
        f,
        [prefix_annotations_before_offset(
            f.context(),
            attribute_id,
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
    let spread_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [prefix_annotations_after_offset(
                f.context(),
                attribute_id,
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

/// Write one tree spread child expression container.
fn write_tree_spread_child<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let node_span = f.context().span(child_id);
    let node_start = f.context().node_token_start(child_id);
    let value_span = f.context().span(value);
    write!(
        f,
        [prefix_annotations_before_offset(
            f.context(),
            child_id,
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
    let spread_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [prefix_annotations_after_offset(
                f.context(),
                child_id,
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
    f: &mut DestackFormatter<'ast, '_>,
    value: &TreeAttributeValue,
) -> FormatResult<()> {
    match value {
        TreeAttributeValue::String(string_id) => {
            let span = Span::empty(f.context().file.id);
            write!(f, [token("=")])?;
            format_scalar_literal(&ScalarLiteral::String(*string_id), span, f)
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
    f: &mut DestackFormatter<'ast, '_>,
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
            write_tree_spread_attribute(f, attribute_id, *value)?;
        }
        TreeAttribute::Error => {
            write!(f, [prefix_annotations(f.context(), attribute_id)])?;
            write!(f, [token("{"), token("/* ERROR */"), token("}")])?;
        }
    }

    if let Some(following_span_start) = following_span_start {
        let enclosing_span =
            tree_node_enclosing_span(f.context(), attribute_id.id, NodeType::TreeAttribute);
        let trailing_span = f.context().span(attribute_id);
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                following_span_start,
            )]
        )?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), attribute_id)])
}

/// Write one tree child.
pub(crate) fn write_tree_child<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    child_id: LocalNodeId<TreeChild>,
    following_span_start: Option<u32>,
) -> FormatResult<()> {
    let mut child_annotations_rendered_inline = false;
    match f.context().tree.get(child_id) {
        TreeChild::Text { value } => {
            write!(f, [text(f.context().strings.get(*value))])?;
        }
        TreeChild::Expression { value } => {
            child_annotations_rendered_inline = write_tree_expression_child(f, child_id, *value)?;
        }
        TreeChild::Spread { value } => {
            write_tree_spread_child(f, child_id, *value)?;
        }
        TreeChild::Tree { value } => {
            write!(f, [prefix_annotations(f.context(), child_id)])?;
            write!(f, [*value])?;
        }
        TreeChild::Error => {
            write!(f, [token("{"), token("/* ERROR */"), token("}")])?;
        }
    }

    if let Some(following_span_start) = following_span_start {
        let enclosing_span =
            tree_node_enclosing_span(f.context(), child_id.id, NodeType::TreeChild);
        let trailing_span = f.context().span(child_id);
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                following_span_start,
            )]
        )?;
    }

    if !child_annotations_rendered_inline {
        write!(f, [infix_or_postfix_annotations(f.context(), child_id)])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, TreeAttribute> for TreeAttribute {
    fn format_node(
        &self,
        node_id: LocalNodeId<TreeAttribute>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _ = self;

        write_tree_attribute(f, node_id, None)
    }
}

impl<'ast> FormatNode<'ast, TreeChild> for TreeChild {
    fn format_node(
        &self,
        node_id: LocalNodeId<TreeChild>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _ = self;

        write_tree_child(f, node_id, None)
    }
}
