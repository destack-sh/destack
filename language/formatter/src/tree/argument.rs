use super::attribute::format_tree_attribute_value;
use super::child::{
    expression_chain_has_separator_comment, format_inline_stub_comments,
    format_multiline_stub_comment_nodes, tree_argument_has_outer_line_comment,
    tree_child_should_inline_braced_expression, tree_control_child_should_expand,
};
use crate::annotation::{
    FormatTrailingComments, format_trailing_comments, infix_or_postfix_annotations,
    prefix_annotations, prefix_annotations_after_offset, prefix_annotations_before_offset,
};
use crate::chain::transparent_inner_expression;
use crate::expression::argument_value;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, Expression, IfCondition, IfForm, LocalNodeId, NodeType, ScalarLiteral,
    TokenType,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_suffix_boundary, soft_block_indent,
    space, token,
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

        let Expression::TreeExpression { elements, .. } = context.tree.get(value_id) else {
            return false;
        };

        elements
            .as_ref()
            .is_some_and(|elements| !elements.is_empty())
    })
}

/// Return whether one tree argument source span is wrapped with `{ ... }`.
pub(crate) fn tree_argument_is_wrapped_in_braces(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.span(argument_id);
    let starts_with_open_brace = context
        .first_non_trivia_token_in_span(argument_span)
        .is_some_and(|token| token.token.ty == TokenType::OpenBrace);
    let ends_with_close_brace = context
        .last_non_trivia_token_in_span(argument_span)
        .is_some_and(|token| token.token.ty == TokenType::CloseBrace);

    starts_with_open_brace && ends_with_close_brace
}

/// Return whether stub prefix docs must stay inside the surrounding `{ ... }`.
fn stub_argument_keeps_prefix_annotations_inside_braces(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    let _ = (context, argument_id, value_id);

    false
}

/// Return stub comment nodes attached to the value span or argument span.
fn stub_argument_comment_nodes(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
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

    let argument_comment_nodes = {
        let comments = context.comments();
        comments
            .comments_in_range(
                context.span(argument_id).start,
                context.span(argument_id).end,
            )
            .to_vec()
    };
    let rendered_inline_from_argument = !argument_comment_nodes.is_empty();

    (argument_comment_nodes, rendered_inline_from_argument)
}

/// Return the enclosing span used for one tree argument's trailing comments.
fn tree_argument_enclosing_span(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Span {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        unreachable!("tree argument should have one parent");
    };

    match parent_type {
        NodeType::Expression => context.span(LocalNodeId::<Expression>::new(parent_id)),
        _ => unreachable!("tree argument parent should be one expression"),
    }
}

/// Write one stub argument inside `{ ... }` and return whether annotations stayed inline.
fn write_stub_tree_expression_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let keep_stub_prefix_inside_braces =
        stub_argument_keeps_prefix_annotations_inside_braces(f.context(), argument_id, value_id);
    if keep_stub_prefix_inside_braces {
        write!(f, [token("{")])?;
        write!(f, [prefix_annotations(f.context(), argument_id)])?;
        write!(f, [prefix_annotations(f.context(), value_id)])?;
        write!(f, [token("}")])?;
        return Ok(true);
    }

    let (comment_nodes, rendered_inline_from_argument) =
        stub_argument_comment_nodes(f.context(), argument_id, value_id);

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
            wrote_stub_comment = format_inline_stub_comments(f, f.context().span(argument_id))?;
        }
        write!(f, [token("}")])?;

        return Ok(rendered_inline_from_argument && wrote_stub_comment);
    }

    Ok(rendered_inline_from_argument)
}

/// Write one tree expression argument.
pub(crate) fn write_tree_expression_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    following_span_start: Option<u32>,
) -> FormatResult<()> {
    let argument = f.context().tree.get(argument_id);
    let argument_is_spread = matches!(argument, Argument::Spread { .. });
    let stub_value_id = match argument {
        Argument::Positional { value, .. }
            if matches!(f.context().tree.get(*value), Expression::Stub) =>
        {
            Some(*value)
        }
        _ => None,
    };

    // non-expression argument comments can stay before the node
    if stub_value_id.is_none()
        && !argument_is_spread
        && !matches!(argument, Argument::Positional { .. })
    {
        write!(f, [prefix_annotations(f.context(), argument_id)])?;
    }

    let mut stub_argument_annotations_rendered_inline = false;
    match argument {
        Argument::Named { name, value, .. } => {
            // named tree attributes preserve their source token form
            write!(f, [name])?;
            let argument_span = f.context().span(argument_id);
            let tokens = f.context().non_trivia_tokens_in_span(argument_span);
            let (is_equals_braced, is_equals_unbraced) = tokens
                .iter()
                .position(|token| token.token.ty == TokenType::Assign)
                .map(|assign_index| {
                    if tokens
                        .get(assign_index + 1)
                        .is_some_and(|token| token.token.ty == TokenType::OpenBrace)
                    {
                        (true, false)
                    } else {
                        (false, true)
                    }
                })
                .unwrap_or((false, false));
            if is_equals_braced {
                format_tree_attribute_value(f, *value)?;
            } else if is_equals_unbraced {
                write!(f, [token("="), value])?;
            }
        }
        Argument::Labeled { label, value, .. } => {
            // label
            write!(f, [label])?;
            // value
            write!(f, [token(":"), space(), value])?;
        }
        Argument::Positional { value, .. } => {
            // in tree expressions, expression children need braces too
            let value_expr = f.context().tree.get(*value);
            let argument_span = f.context().span(argument_id);
            let argument_is_braced = tree_argument_is_wrapped_in_braces(f.context(), argument_id);
            let needs_braces = argument_is_braced
                || !matches!(
                    value_expr,
                    Expression::ScalarLiteral(ScalarLiteral::String(_))
                        | Expression::TreeExpression { .. }
                );
            if needs_braces {
                let value_span = f.context().span(*value);

                let trailing_comments = |f: &DestackFormatter<'ast, '_>| {
                    f.context()
                        .comments()
                        .comments_before(argument_span.end)
                        .iter()
                        .copied()
                        .filter(|comment| comment.span.start >= value_span.end)
                        .collect::<Vec<_>>()
                };
                let force_multiline_braced_expression =
                    tree_argument_has_outer_line_comment(f.context(), argument_id, *value);

                if matches!(value_expr, Expression::Stub) {
                    stub_argument_annotations_rendered_inline =
                        write_stub_tree_expression_argument(f, argument_id, *value)?;
                } else if force_multiline_braced_expression {
                    write!(
                        f,
                        [group(&format_args![
                            token("{"),
                            prefix_annotations(f.context(), argument_id),
                            block_indent(&format_with(|f| {
                                write!(f, [group(value).should_expand(true)])?;
                                let trailing_comments = trailing_comments(f);
                                write!(f, [FormatTrailingComments::Comments(&trailing_comments)])
                            })),
                            hard_line_break(),
                            token("}")
                        ])]
                    )?;
                } else {
                    // keep tree expression containers inline for common expression forms
                    if tree_child_should_inline_braced_expression(f.context(), argument_id) {
                        write!(
                            f,
                            [
                                token("{"),
                                prefix_annotations(f.context(), argument_id),
                                value,
                                FormatTrailingComments::Comments(&trailing_comments(f)),
                                token("}")
                            ]
                        )?;
                    } else if expression_chain_has_separator_comment(f.context(), *value)
                        || tree_control_child_should_expand(f.context(), *value)
                        || matches!(
                            f.context().tree.get(*value),
                            Expression::If {
                                form: IfForm::Ternary,
                                condition,
                                then_expression,
                                else_expression,
                                ..
                            } if {
                                let condition_has_line_comment = match condition {
                                    IfCondition::Expression { condition } => {
                                        node_has_line_comment(f.context(), *condition)
                                    }
                                    IfCondition::Let { .. } => false,
                                };

                                condition_has_line_comment
                                    || node_has_line_comment(f.context(), *then_expression)
                                    || else_expression.is_some_and(|expression_id| {
                                        node_has_line_comment(f.context(), expression_id)
                                    })
                            }
                        )
                    {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                prefix_annotations(f.context(), argument_id),
                                group(value).should_expand(true),
                                FormatTrailingComments::Comments(&trailing_comments(f)),
                                token("}")
                            ])]
                        )?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                soft_block_indent(&format_with(|f| {
                                    write!(f, [prefix_annotations(f.context(), argument_id)])?;
                                    write!(f, [value])?;
                                    let trailing_comments = trailing_comments(f);
                                    write!(
                                        f,
                                        [FormatTrailingComments::Comments(&trailing_comments)]
                                    )
                                })),
                                token("}")
                            ])]
                        )?;
                    }
                }
            } else {
                write!(f, [value])?;
            }
        }
        Argument::Spread { value, .. } => {
            let argument_span = f.context().span(argument_id);
            let argument_start = f.context().node_token_start(argument_id);
            let value_span = f.context().span(*value);
            write!(
                f,
                [prefix_annotations_before_offset(
                    f.context(),
                    argument_id,
                    argument_start
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
                    .comments_in_range(value_span.end, argument_span.end)
                    .is_empty();
            let spread_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [prefix_annotations_after_offset(
                        f.context(),
                        argument_id,
                        argument_start
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
        }
        Argument::Error => {
            write!(f, [token("{"), token("/* ERROR */"), token("}")])?;
        }
    }

    if let Some(following_span_start) = following_span_start {
        let enclosing_span = tree_argument_enclosing_span(f.context(), argument_id);
        let trailing_span = f.context().span(argument_id);
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                following_span_start,
            )]
        )?;
    }

    if !stub_argument_annotations_rendered_inline {
        write!(f, [infix_or_postfix_annotations(f.context(), argument_id)])?;
    }

    Ok(())
}
