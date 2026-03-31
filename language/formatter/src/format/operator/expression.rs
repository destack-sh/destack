use crate::format::call::{
    format_call_arguments, format_call_expression, format_instantiation_expression,
};
use crate::format::chain::{
    call_has_parenthesized_await_member_receiver, extract_parenthesized_index_chain,
    format_expression_chain, format_maybe_expression, has_chain_parent, is_expression_chain,
};
use crate::format::expression::{
    expression_has_leading_prefix_comment, format_index_expression, format_member_expression,
    format_static_argument_list, member_expression_has_optional_chain,
    member_object_prefers_new_callee_parentheses,
};
use crate::format::operator::assign::format_assign_expression;
use crate::format::operator::binary::format_binary_expression;
use crate::format::operator::needs_parens_in_postfix_position;
use crate::format::operator::types::format_type_binary_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, CommentStyle, Expression, LocalNodeId, Mutability, PostfixPosition,
    TokenType, TypeUnaryOperator, UnaryOperator,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{format_with, hard_line_break, soft_block_indent, space, token};
use destack_fir::write;

/// Return whether a member object is simple enough for `new a.b()` style callee formatting.
fn is_simple_new_member_object(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_simple_new_member_object(tree, *left)
        }
        Expression::Parenthesized { expression } => is_simple_new_member_object(tree, *expression),
        _ => false,
    }
}

/// Decide whether `new (<member>)()` can unwrap outer parentheses.
fn should_unwrap_parenthesized_new_member_callee(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if crate::format::expression::parenthesized_has_leading_inner_trivia(
        context,
        parenthesized_id,
        inner_expression_id,
    ) {
        return false;
    }

    match context.tree.get(inner_expression_id) {
        Expression::Path { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            if member_expression_has_optional_chain(context, inner_expression_id) {
                return false;
            }

            is_simple_new_member_object(context.tree, *left)
        }
        _ => false,
    }
}

/// Return whether one operator expression serializes infix seams as postfix-only annotations.
pub(crate) fn operator_expression_uses_postfix_only_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    if matches!(
        expression,
        Expression::TypeUnary {
            operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
            ..
        }
    ) {
        return true;
    }

    matches!(
        expression,
        Expression::Call {
            dynamic_arguments,
            ..
        }
        | Expression::New {
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && context.has_infix_annotation(node_id)
    )
}

/// Write trailing annotations for one operator expression.
pub(crate) fn write_operator_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    // postfix-style operator seams own their infix serialization locally
    if operator_expression_uses_postfix_only_annotations(f.context(), expression_id, expression) {
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                expression_id
            )]
        )?;
        return Ok(());
    }

    write!(
        f,
        [crate::format::annotation::infix_or_postfix_annotations(
            f.context(),
            expression_id
        )]
    )
}

/// Format a `new` expression.
pub(crate) fn format_new_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // unwrap redundant parenthesized member callees
    let mut left = left;
    if let Expression::Parenthesized { expression } = tree.get(left)
        && should_unwrap_parenthesized_new_member_callee(f.context(), left, *expression)
    {
        left = *expression;
    }

    // preserve parenthesized index chains like `(foo[bar])`
    if let Some((base_expression, indices)) =
        extract_parenthesized_index_chain(f.context().tree, left)
    {
        let formatted_left = format_with(move |f| {
            write!(f, [token("("), base_expression])?;
            for index in &indices {
                write!(f, [token("["), *index, token("]")])?;
            }
            write!(f, [token(")")])
        });

        write!(f, [token("new"), space(), formatted_left])?;
    }
    // otherwise use regular member-callee wrapping rules
    else {
        let should_wrap_member_callee = matches!(
            tree.get(left),
            Expression::Member {
                left: member_left,
                ..
            } | Expression::PrivateMember {
                left: member_left,
                ..
            } if member_object_prefers_new_callee_parentheses(
                f.context(),
                *member_left
            )
        );

        // keep wrapped member callee when required
        if should_wrap_member_callee {
            write!(f, [token("new"), space(), token("("), left, token(")")])?;
        }
        // otherwise write regular new callee
        else {
            write!(f, [token("new"), space(), left])?;
        }
    }

    // write static type arguments
    if let Some(static_arguments) = static_arguments {
        format_static_argument_list(f, static_arguments)?;
    }

    // write dynamic argument list
    format_call_arguments(f, node_id, dynamic_arguments)?;

    Ok(())
}

/// Collect block infix comment nodes for one type unary expression node.
fn type_unary_infix_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<(CommentStyle, bool, LocalNodeId<Comment>)> {
    let Expression::TypeUnary { right, .. } = context.tree.get(node_id) else {
        return Vec::new();
    };

    let right_span = context.span(*right);
    let node_span = context.span(node_id);
    if right_span.file != node_span.file || right_span.end >= node_span.end {
        return Vec::new();
    }

    context
        .comment_nodes_in_range(right_span.end, node_span.end)
        .into_iter()
        .map(|comment_id| {
            let comment = context.tree.get(comment_id);
            let comment_span = context.span(comment_id);
            (
                comment.style,
                context.span_has_newline_before_next_non_whitespace_token(comment_span),
                comment_id,
            )
        })
        .collect()
}

/// Write a type-unary `as <keyword>` suffix with infix comment seams.
fn write_type_unary_as_keyword_with_infix_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    keyword: &'static str,
) -> FormatResult<()> {
    let infix_comments = type_unary_infix_comments(f.context(), node_id);
    if infix_comments.is_empty() {
        write!(f, [space(), token(keyword)])?;
        return Ok(());
    }

    if infix_comments.len() == 1
        && infix_comments[0].0 == CommentStyle::Slash
        && !infix_comments[0].1
    {
        write!(
            f,
            [
                space(),
                infix_comments[0].2,
                hard_line_break(),
                token(keyword)
            ]
        )?;
        return Ok(());
    }

    if infix_comments.len() == 1
        && infix_comments[0].0 == CommentStyle::Star
        && !infix_comments[0].1
    {
        write!(f, [space(), infix_comments[0].2, space(), token(keyword)])?;
        return Ok(());
    }

    write!(f, [hard_line_break()])?;
    for (comment_index, (_, _, comment_id)) in infix_comments.iter().enumerate() {
        if comment_index > 0 {
            write!(f, [hard_line_break()])?;
        }
        write!(f, [*comment_id])?;
    }
    write!(f, [hard_line_break(), token(keyword)])
}

/// Return whether this type-unary node should keep TypeScript angle assertion syntax.
fn type_unary_prefers_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    context
        .first_non_trivia_token_in_span(main_span)
        .is_some_and(|token| token.token.ty == TokenType::LessThan)
}

/// Format operator and chain expression variants.
pub(crate) fn format_operator_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                let right_needs_await_or_yield_grouping = matches!(
                    tree.get(*right),
                    Expression::Await { .. }
                        | Expression::AwaitMaybe { .. }
                        | Expression::Yield { .. }
                );
                let right_has_leading_prefix_comment =
                    expression_has_leading_prefix_comment(f.context(), *right);
                let right_is_parenthesized =
                    matches!(tree.get(*right), Expression::Parenthesized { .. });
                let right_needs_comment_grouping =
                    right_has_leading_prefix_comment && !right_is_parenthesized;
                let right_needs_inline_grouping = right_needs_await_or_yield_grouping;
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                if needs_space {
                    if right_needs_comment_grouping {
                        write!(
                            f,
                            [
                                operator,
                                space(),
                                token("("),
                                soft_block_indent(right),
                                token(")")
                            ]
                        )?;
                    } else if right_needs_inline_grouping {
                        write!(f, [operator, space(), token("("), right, token(")")])?;
                    } else {
                        write!(f, [operator, space(), right])?;
                    }
                } else if right_needs_comment_grouping {
                    write!(
                        f,
                        [operator, token("("), soft_block_indent(right), token(")")]
                    )?;
                } else if right_needs_inline_grouping {
                    write!(f, [operator, token("("), right, token(")")])?;
                } else {
                    write!(f, [operator, right])?;
                }
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type unary
        Expression::TypeUnary { operator, right } => match operator {
            TypeUnaryOperator::Not => {
                write!(f, [operator, right])?;
            }
            TypeUnaryOperator::Must => {
                write!(f, [right, operator])?;
            }
            TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof => {
                write!(f, [operator, space(), right])?;
            }
            TypeUnaryOperator::AsConst => {
                if type_unary_prefers_angle_assertion_syntax(f.context(), node_id) {
                    write!(f, [token("<const>"), right])?;
                    return Ok(true);
                }

                let right_has_postfix = f.context().has_postfix_annotation(*right);
                write!(f, [right])?;
                if right_has_postfix {
                    write!(f, [token("as")])?;
                } else {
                    write!(f, [token(" as")])?;
                }
                write_type_unary_as_keyword_with_infix_comments(f, node_id, "const")?;
            }
            TypeUnaryOperator::AsComptime => {
                let right_has_postfix = f.context().has_postfix_annotation(*right);
                write!(f, [right])?;
                if right_has_postfix {
                    write!(f, [token("as")])?;
                } else {
                    write!(f, [token(" as")])?;
                }
                write_type_unary_as_keyword_with_infix_comments(f, node_id, "comptime")?;
            }
        },

        // value
        Expression::ValueOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("^")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // await
        Expression::Await { expression } => {
            write!(f, [token("await"), space(), expression])?;
        }

        // await?
        Expression::AwaitMaybe { expression } => {
            write!(f, [token("await"), token("?"), space(), expression])?;
        }

        // comptime
        Expression::Comptime { body } => {
            write!(f, [token("comptime"), space(), body])?;
        }

        // reference
        Expression::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("&")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // pointer
        Expression::PointerOf { mutability, right } => {
            write!(f, [token("*")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            right.format(f)?;
        }

        // member
        Expression::Member { .. } | Expression::PrivateMember { .. } => {
            format_member_or_chain_expression(f, node_id)?;
        }

        // index
        Expression::Index { .. } => {
            format_index_or_chain_expression(f, node_id)?;
        }

        // call
        Expression::Call { .. } => {
            format_call_or_chain_expression(f, node_id)?;
        }

        // instantiation
        Expression::Instantiation { .. } => {
            format_instantiation_or_chain_expression(f, node_id)?;
        }

        // new
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            format_new_expression(f, node_id, *left, static_arguments, dynamic_arguments)?;
        }

        // delete
        Expression::Delete { value } => {
            write!(f, [token("delete"), space(), value])?;
        }

        // maybe
        Expression::Maybe { .. } => {
            format_maybe_or_chain_expression(f, node_id)?;
        }

        // must
        Expression::Must { position, left } => {
            format_must_or_chain_expression(f, node_id, *position, *left)?;
        }

        // binary
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            format_binary_expression(f, node_id, *left, operator, *right)?;
        }

        // type binary
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            format_type_binary_expression(f, node_id, *left, operator, *right)?;
        }

        // assign
        Expression::Assign {
            left,
            operator,
            right,
        } => {
            format_assign_expression(f, node_id, *left, operator, *right)?;
        }

        // debugger
        Expression::Debugger => {
            write!(f, [token("debugger")])?;
        }

        // stub: placeholder for annotation only files
        Expression::Stub => {}

        // missing: preserve the surrounding syntax hole
        Expression::Missing => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Format a member expression or route to chain formatting.
fn format_member_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        format_expression_chain(f, node_id)?;
    } else {
        format_member_expression(f, node_id)?;
    }

    Ok(())
}

/// Format an index expression or route to chain formatting.
fn format_index_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        format_expression_chain(f, node_id)?;
    } else {
        format_index_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a call expression or route to chain formatting.
fn format_call_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let call_has_await_wrapped_member_receiver =
        call_has_parenthesized_await_member_receiver(f.context(), node_id);
    let should_route_to_chain =
        is_expression_chain(f.context().tree, node_id) && !call_has_await_wrapped_member_receiver;
    if should_route_to_chain {
        format_expression_chain(f, node_id)?;
    } else {
        format_call_expression(f, node_id)?;
    }

    Ok(())
}

/// Format an instantiation expression or route to chain formatting.
fn format_instantiation_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) && has_chain_parent(f.context(), node_id) {
        format_expression_chain(f, node_id)?;
    } else {
        format_instantiation_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a maybe expression or route to chain formatting.
fn format_maybe_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        format_expression_chain(f, node_id)?;
    } else {
        format_maybe_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a must expression or route to chain formatting.
fn format_must_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    position: PostfixPosition,
    left: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        format_expression_chain(f, node_id)?;
    } else {
        let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, left);
        if needs_parentheses {
            write!(f, [token("("), left, token(")")])?;
        } else {
            write!(f, [left])?;
        }
        if position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        write!(f, [token("!")])?;
    }

    Ok(())
}
