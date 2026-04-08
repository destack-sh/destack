use crate::format::annotation::{
    format_raw_comment, infix_or_postfix_annotations, postfix_annotations,
};
use crate::format::call::{
    call_should_route_to_chain, format_call_expression, format_instantiation_expression,
    format_new_expression,
};
use crate::format::chain::{format_expression_chain, format_maybe_expression};
use crate::format::expression::{
    expression_has_leading_prefix_comment, format_index_expression, format_member_expression,
};
use crate::format::operator::assign::format_assign_expression;
use crate::format::operator::binary::format_binary_expression;
use crate::format::operator::needs_parens_in_postfix_position;
use crate::format::operator::types::{
    expression_uses_angle_assertion_syntax, format_type_binary_expression,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Comment, CommentKind, Expression, LocalNodeId, Mutability, PostfixPosition, TypeUnaryOperator,
    UnaryOperator,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{hard_line_break, soft_block_indent, space, token};
use destack_fir::write;

/// Return whether one operator expression serializes infix annotations as postfix-only annotations.
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
    // postfix-style operator expressions own their infix serialization locally
    if operator_expression_uses_postfix_only_annotations(f.context(), expression_id, expression) {
        write!(f, [postfix_annotations(f.context(), expression_id)])?;
        return Ok(());
    }

    write!(
        f,
        [infix_or_postfix_annotations(f.context(), expression_id)]
    )
}

/// Collect block infix comment nodes for one type unary expression node.
fn type_unary_infix_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<(CommentKind, bool, Comment)> {
    let Expression::TypeUnary { right, .. } = context.tree.get(node_id) else {
        return Vec::new();
    };

    let right_span = context.span(*right);
    let node_span = context.span(node_id);
    if right_span.file != node_span.file || right_span.end >= node_span.end {
        return Vec::new();
    }

    context
        .comments()
        .comments_in_range(right_span.end, node_span.end)
        .iter()
        .copied()
        .map(|comment| {
            let comment_span = comment.span;
            (
                comment.kind,
                context.span_has_newline_before_next_non_whitespace_token(comment_span),
                comment,
            )
        })
        .collect()
}

/// Write a type-unary `as <keyword>` suffix with infix comments.
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

    if infix_comments.len() == 1 && infix_comments[0].0 == CommentKind::Line && !infix_comments[0].1
    {
        write!(f, [space()])?;
        format_raw_comment(f, infix_comments[0].2)?;
        write!(f, [hard_line_break(), token(keyword)])?;
        return Ok(());
    }

    if infix_comments.len() == 1 && infix_comments[0].0 != CommentKind::Line && !infix_comments[0].1
    {
        write!(f, [space()])?;
        format_raw_comment(f, infix_comments[0].2)?;
        write!(f, [space(), token(keyword)])?;
        return Ok(());
    }

    write!(f, [hard_line_break()])?;
    for (comment_index, (_, _, comment)) in infix_comments.iter().enumerate() {
        if comment_index > 0 {
            write!(f, [hard_line_break()])?;
        }
        format_raw_comment(f, *comment)?;
    }
    write!(f, [hard_line_break(), token(keyword)])
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
                if expression_uses_angle_assertion_syntax(f.context(), node_id, false) {
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
            format_member_expression(f, node_id)?;
        }

        // index
        Expression::Index { .. } => {
            format_index_expression(f, node_id)?;
        }

        // call
        Expression::Call { .. } => {
            format_call_or_chain_expression(f, node_id)?;
        }

        // instantiation
        Expression::Instantiation { .. } => {
            format_instantiation_expression(f, node_id)?;
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
            format_maybe_expression(f, node_id)?;
        }

        // must
        Expression::Must { position, left } => {
            format_must_expression(f, *position, *left)?;
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

/// Format a call expression or route to chain formatting.
fn format_call_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let Expression::Call {
        left,
        dynamic_arguments,
        ..
    } = f.context().tree.get(node_id)
    else {
        debug_assert!(false, "unexpected expression kind for call-chain routing");
        return Ok(());
    };

    let should_route_to_chain =
        call_should_route_to_chain(f.context(), node_id, *left, dynamic_arguments);
    if should_route_to_chain {
        format_expression_chain(f, node_id)?;
    } else {
        format_call_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a must expression without chain routing.
fn format_must_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    position: PostfixPosition,
    left: LocalNodeId<Expression>,
) -> FormatResult<()> {
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

    Ok(())
}
