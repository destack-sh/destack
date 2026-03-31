use super::r#type::write_expression_with_inline_prefix_annotations;
use crate::format::chain::{is_chain_root, is_expression_chain, transparent_inner_expression};
use crate::format::expression::expression_has_leading_prefix_comment;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Expression, LocalNodeId, NodeType, TokenType, TypeBinaryOperator};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, indent, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};

/// Return whether one type binary left expression is simple enough to stay ungrouped.
pub(crate) fn is_simple_type_binary_left_expression(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            is_simple_type_binary_left_expression(tree, *expression)
        }
        Expression::TypeBinary { left, operator, .. } => {
            matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && is_simple_type_binary_left_expression(tree, *left)
        }
        Expression::Path { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Call { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. }
        | Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_) => true,
        _ => false,
    }
}

/// Return whether one type expression is object-like.
pub(crate) fn is_object_like_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Return whether one expression appears inside a type template literal interpolation.
pub(crate) fn is_in_type_template_literal_interpolation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;

    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type == NodeType::Expression
            && matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::TypeTemplateLiteral { .. }
            )
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Write a cast or satisfies operator and right operand.
fn write_type_binary_operator_and_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [operator, space()])?;
    write_expression_with_inline_prefix_annotations(f, right)
}

/// Return whether this cast expression should keep TypeScript angle assertion syntax.
fn cast_prefers_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let mut token_index = 0usize;
    while let Some(token) = context.nth_non_trivia_token_in_span(main_span, token_index) {
        match token.token.ty {
            TokenType::OpenParenthesis => token_index += 1,
            TokenType::LessThan => return true,
            _ => return false,
        }
    }

    false
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(crate) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && cast_prefers_angle_assertion_syntax(f.context(), node_id);

    let mut formatted_left = left;
    if !cast_uses_angle_assertion
        && matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        )
        && let Expression::Parenthesized { expression } = f.context().tree.get(left)
        && should_drop_type_binary_left_parentheses(f.context(), node_id, left, *expression)
    {
        formatted_left = *expression;
    }

    // statement-level satisfies/cast over object literals should keep `({ ... })` lhs wrapping
    let left_needs_statement_object_parentheses = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && matches!(
        f.context().tree.get(formatted_left),
        Expression::ObjectExpression { .. }
    ) && f.context().parent(node_id).is_some_and(
        |(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            if matches!(
                f.context().tree.get(parent_expression_id),
                Expression::Statement(inner) if *inner == node_id
            ) {
                return true;
            }

            let Expression::Parenthesized { expression } =
                f.context().tree.get(parent_expression_id)
            else {
                return false;
            };
            if *expression != node_id {
                return false;
            }

            f.context().parent(parent_expression_id).is_some_and(
                |(grandparent_id, grandparent_type)| {
                    grandparent_type == NodeType::Expression
                        && matches!(
                            f.context()
                                .tree
                                .get(LocalNodeId::<Expression>::new(grandparent_id)),
                            Expression::Statement(inner_id) if *inner_id == parent_expression_id
                        )
                },
            )
        },
    );

    let format_left = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        if left_needs_statement_object_parentheses {
            write!(f, [token("("), formatted_left, token(")")])
        } else {
            write!(f, [formatted_left])
        }
    };

    if cast_uses_angle_assertion {
        if f.context().has_prefix_annotation(right) {
            let format_cast = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [token("<"), group(&soft_block_indent(&right)), token(">")]
                )
            });
            write!(f, [format_cast, format_with(format_left)])?;
        } else {
            let should_preserve_parenthesized_left_newline =
                matches!(
                    f.context().tree.get(formatted_left),
                    Expression::Parenthesized { .. }
                ) && f.context().node_has_newline(formatted_left);
            if should_preserve_parenthesized_left_newline
                && let Expression::Parenthesized { expression } =
                    f.context().tree.get(formatted_left)
            {
                write!(
                    f,
                    [
                        token("<"),
                        right,
                        token(">"),
                        token("("),
                        soft_block_indent(expression),
                        token(")")
                    ]
                )?;
            } else {
                write!(f, [token("<"), right, token(">"), format_with(format_left)])?;
            }
        }
        return Ok(());
    }

    let left_has_leading_prefix_comment =
        expression_has_leading_prefix_comment(f.context(), formatted_left);
    let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
        || is_chain_root(f.context().tree, formatted_left);
    let is_parenthesized_new_callee =
        f.context()
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                let Expression::Parenthesized { expression } =
                    f.context().tree.get(parent_expression_id)
                else {
                    return false;
                };
                if *expression != node_id {
                    return false;
                }

                f.context().parent(parent_expression_id).is_some_and(
                    |(grandparent_id, grandparent_type)| {
                        grandparent_type == NodeType::Expression
                            && matches!(
                                f.context()
                                    .tree
                                    .get(LocalNodeId::<Expression>::new(grandparent_id)),
                                Expression::New { left, .. } if *left == parent_expression_id
                            )
                    },
                )
            });

    let should_expand_chain_left = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && left_is_chain_expression
        && is_parenthesized_new_callee;

    let is_parenthesized_member_object_or_call_callee = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && f
        .context()
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let Expression::Parenthesized { expression } =
                f.context().tree.get(parent_expression_id)
            else {
                return false;
            };
            if *expression != node_id {
                return false;
            }

            f.context().parent(parent_expression_id).is_some_and(
                |(grandparent_id, grandparent_type)| {
                    if grandparent_type != NodeType::Expression {
                        return false;
                    }

                    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
                    match f.context().tree.get(grandparent_expression_id) {
                        Expression::Member { left, .. }
                        | Expression::PrivateMember { left, .. }
                        | Expression::Index { left, .. } => *left == parent_expression_id,
                        Expression::Call { left, .. } => *left == parent_expression_id,
                        _ => false,
                    }
                },
            )
        });

    if is_parenthesized_member_object_or_call_callee {
        write!(
            f,
            [group(&soft_block_indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    format_left(f)?;
                    write!(f, [space()])?;
                    write_type_binary_operator_and_right(f, operator, right)
                }
            )))]
        )?;
        return Ok(());
    }

    if should_expand_chain_left {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        format_left(f)
                    }))
                    .should_expand(true)]
                )?;
                write!(f, [space()])?;
                write_type_binary_operator_and_right(f, operator, right)
            }))]
        )?;
    } else {
        let keep_left_and_operator_on_same_line = matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        );

        write!(
            f,
            [group(&format_args![
                format_with(format_left),
                indent(&format_with(|f| {
                    if keep_left_and_operator_on_same_line || left_has_leading_prefix_comment {
                        write!(f, [space()])?;
                    } else {
                        write!(f, [soft_line_break_or_space()])?;
                    }
                    write_type_binary_operator_and_right(f, operator, right)
                }))
            ])]
        )?;
    }

    Ok(())
}

/// Decide whether cast or satisfies can drop a parenthesized left side.
fn should_drop_type_binary_left_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parenthesized_id: LocalNodeId<Expression>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    let left_is_cast_chain = matches!(
        context.tree.get(left_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_cast_chain {
        return false;
    }

    if crate::format::expression::parenthesized_has_leading_inner_trivia(
        context,
        parenthesized_id,
        left_id,
    ) && !left_is_cast_chain
    {
        return false;
    }

    if context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            if matches!(
                context.tree.get(parent_expression_id),
                Expression::Statement(inner) if *inner == node_id
            ) {
                return true;
            }

            let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id)
            else {
                return false;
            };
            if *expression != node_id {
                return false;
            }

            context.parent(parent_expression_id).is_some_and(
                |(grandparent_id, grandparent_type)| {
                    grandparent_type == NodeType::Expression
                        && matches!(
                            context.tree.get(LocalNodeId::<Expression>::new(grandparent_id)),
                            Expression::Statement(inner_id) if *inner_id == parent_expression_id
                        )
                },
            )
        })
    {
        return false;
    }

    let mut current_id = node_id;
    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        let has_parenthesized_ancestor_with_leading_inner_trivia =
            if let Expression::Parenthesized { expression } = context.tree.get(parent_id) {
                *expression == current_id
                    && crate::format::expression::parenthesized_has_leading_inner_trivia(
                        context, parent_id, current_id,
                    )
            } else {
                false
            };
        if has_parenthesized_ancestor_with_leading_inner_trivia && !left_is_cast_chain {
            return false;
        }

        current_id = parent_id;
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}
