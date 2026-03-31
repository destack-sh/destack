use super::attribute::format_tree_attribute_value;
use super::child::{
    expression_has_chain_seam_comment, format_inline_stub_comments,
    format_multiline_stub_comment_nodes, node_has_line_comment,
    tree_child_should_inline_braced_expression,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::declaration::expression_is_decorated_class_declaration;
use crate::format::expression::argument_value;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, IfCondition, IfKind,
    LocalNodeId, ScalarLiteral, TokenType,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_postfix_boundary, soft_block_indent,
    space, token,
};
use destack_fir::{format_args, write};

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

        context.node_has_newline(argument_id)
            || context.node_has_newline(value_id)
            || elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty())
    })
}

/// Decide whether an argument can drop one parenthesized value wrapper.
pub(crate) fn argument_drops_parenthesized_value_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let inner_expression = context.tree.get(inner_expression_id);

    let drops_lambda_wrapper = !context.has_annotation(parenthesized_id)
        && !context.has_annotation(inner_expression_id)
        && !crate::format::expression::parenthesized_has_leading_inner_trivia(
            context,
            parenthesized_id,
            inner_expression_id,
        )
        && matches!(
            inner_expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    context.tree.get(*declaration_id),
                    Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                )
        );

    let drops_decorated_class_wrapper = !context.has_annotation(parenthesized_id)
        && !crate::format::expression::parenthesized_has_leading_inner_newline(
            context,
            parenthesized_id,
            inner_expression_id,
        )
        && expression_is_decorated_class_declaration(context, inner_expression_id);

    drops_lambda_wrapper || drops_decorated_class_wrapper
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

/// Write one tree expression argument.
pub(crate) fn write_tree_expression_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
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

    // stub argument prefix comments and docs must stay inside `{ ... }`
    if stub_value_id.is_none() && !argument_is_spread {
        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                argument_id
            )]
        )?;
    }

    let mut stub_argument_annotations_rendered_inline = false;
    match argument {
        Argument::Named { name, value, .. } => {
            // destack tree literals normalize boolean true and string literal attribute values
            if f.context().options.language_type.is_destack() {
                let value_expr = f.context().tree.get(*value);
                if let Expression::ScalarLiteral(ScalarLiteral::Boolean(true)) = value_expr {
                    write!(f, [name])?;
                } else {
                    write!(f, [name])?;
                    let is_string_literal = matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::ScalarLiteral(ScalarLiteral::Character(_))
                    );
                    if is_string_literal {
                        write!(f, [token("="), value])?;
                    } else {
                        format_tree_attribute_value(f, *value)?;
                    }
                }
            } else {
                // tsx and jsx preserve named attribute token syntax
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
            let argument_is_braced = tree_argument_is_wrapped_in_braces(f.context(), argument_id);
            let force_multiline_braced_expression = node_has_line_comment(f.context(), argument_id)
                || node_has_line_comment(f.context(), *value);
            let needs_braces = argument_is_braced
                || !matches!(
                    value_expr,
                    Expression::ScalarLiteral(ScalarLiteral::String(_))
                        | Expression::TreeExpression { .. }
                );
            if needs_braces {
                if matches!(value_expr, Expression::Stub) {
                    let keep_stub_prefix_inside_braces = stub_value_id.is_some_and(|value_id| {
                        f.context().annotation_ids(argument_id).iter().copied().any(
                            |annotation_id| {
                                let annotation = f.context().annotation(annotation_id);
                                matches!(annotation, Annotation::Doc { .. })
                                    && matches!(
                                        annotation.position(),
                                        AnnotationPosition::BlockPrefix
                                            | AnnotationPosition::LinePrefix
                                    )
                            },
                        ) || f.context().annotation_ids(value_id).iter().copied().any(
                            |annotation_id| {
                                let annotation = f.context().annotation(annotation_id);
                                matches!(annotation, Annotation::Doc { .. })
                                    && matches!(
                                        annotation.position(),
                                        AnnotationPosition::BlockPrefix
                                            | AnnotationPosition::LinePrefix
                                    )
                            },
                        )
                    });
                    if keep_stub_prefix_inside_braces {
                        write!(f, [token("{")])?;
                        write!(
                            f,
                            [crate::format::annotation::prefix_annotations(
                                f.context(),
                                argument_id
                            )]
                        )?;
                        write!(
                            f,
                            [crate::format::annotation::prefix_annotations(
                                f.context(),
                                *value
                            )]
                        )?;
                        write!(f, [token("}")])?;
                        stub_argument_annotations_rendered_inline = true;
                    } else {
                        let expression_comment_nodes = f.context().comment_nodes_in_range(
                            f.context().span(*value).start,
                            f.context().span(*value).end,
                        );
                        let argument_comment_nodes = f.context().comment_nodes_in_range(
                            f.context().span(argument_id).start,
                            f.context().span(argument_id).end,
                        );

                        let mut comment_nodes = expression_comment_nodes;
                        if comment_nodes.is_empty() {
                            comment_nodes = argument_comment_nodes;
                            if !comment_nodes.is_empty() {
                                stub_argument_annotations_rendered_inline = true;
                            }
                        }

                        if comment_nodes.iter().any(|comment_id| {
                            f.context()
                                .tree
                                .get::<destack_ast::Comment>(*comment_id)
                                .style
                                == destack_ast::CommentStyle::Slash
                        }) {
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
                            let mut wrote_stub_comment =
                                format_inline_stub_comments(f, f.context().span(*value))?;
                            if !wrote_stub_comment {
                                wrote_stub_comment =
                                    format_inline_stub_comments(f, f.context().span(argument_id))?;
                                if wrote_stub_comment {
                                    stub_argument_annotations_rendered_inline = true;
                                }
                            }
                            write!(f, [token("}")])?;
                        }
                    }
                } else if force_multiline_braced_expression {
                    write!(
                        f,
                        [group(&format_args![
                            token("{"),
                            block_indent(&group(value).should_expand(true)),
                            hard_line_break(),
                            token("}")
                        ])]
                    )?;
                } else {
                    // keep jsx expression containers inline for common expression forms
                    if tree_child_should_inline_braced_expression(f.context(), argument_id) {
                        write!(f, [token("{"), value, token("}")])?;
                    } else if expression_has_chain_seam_comment(f.context(), *value)
                        || matches!(
                            f.context().tree.get(*value),
                            Expression::If {
                                kind: IfKind::Ternary,
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
                                group(value).should_expand(true),
                                token("}")
                            ])]
                        )?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                soft_block_indent(&value),
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
            // keep spread-head annotations inside `{ ... }` like prettier and oxc
            let argument_span = f.context().span(argument_id);
            let value_span = f.context().span(*value);
            let has_spread_comment = !f
                .context()
                .comments_in_range(argument_span.start, argument_span.end)
                .is_empty()
                || !f
                    .context()
                    .comments_in_range(value_span.start, value_span.end)
                    .is_empty();
            let spread_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [crate::format::annotation::prefix_annotations(
                        f.context(),
                        argument_id
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
                        line_postfix_boundary(),
                        token("}")
                    ])]
                )?;
            } else {
                write!(
                    f,
                    [
                        token("{"),
                        spread_inner,
                        line_postfix_boundary(),
                        token("}")
                    ]
                )?;
            }
        }
        Argument::Error => {
            write!(f, [token("{"), token("/* ERROR */"), token("}")])?;
        }
    }

    if !stub_argument_annotations_rendered_inline {
        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                argument_id
            )]
        )?;
    }

    Ok(())
}
