use crate::format::annotation::{
    format_raw_comment, infix_or_postfix_annotations, postfix_annotations,
};
use crate::format::call::{
    call_should_route_to_chain, format_call_expression, format_instantiation_expression,
    format_new_expression, instantiation_should_route_to_chain,
};
use crate::format::chain::{format_expression_chain, format_maybe_expression};
use crate::format::expression::{
    expression_has_leading_prefix_comment, format_index_expression, format_member_expression,
    write_expression_with_prefix_annotations_after_offset,
};
use crate::format::operator::assign::format_assign_expression;
use crate::format::operator::binary::format_binary_expression;
use crate::format::operator::needs_parens_in_postfix_position;
use crate::format::operator::types::{format_as_expression, format_satisfies_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Comment, Expression, LocalNodeId, Mutability, PostfixPosition, UnaryOperator};
use destack_fir::format::{Buffer, Format, FormatError, FormatResult};
use destack_fir::prelude::{
    empty_line, format_with, hard_line_break, soft_block_indent, space, token,
};
use destack_fir::write;
use destack_source::Span;

/// Return whether one operator expression serializes infix annotations as postfix-only annotations.
pub(crate) fn operator_expression_uses_postfix_only_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    matches!(
        expression,
        Expression::Call {
            arguments,
            ..
        }
        | Expression::New {
            arguments,
            ..
        } if arguments.is_empty() && context.has_infix_annotation(node_id)
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

/// Write grouped prefix-unary operand comments with source-shaped operand spacing.
fn write_grouped_prefix_unary_operand_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
    right_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if comments.is_empty() {
        return Ok(());
    }

    let source = f.context().source_text();
    let mut previous_comment: Option<Comment> = None;

    for comment in comments.iter().copied() {
        if let Some(previous_comment) = previous_comment {
            let lines_before = {
                let comment_cursor = f.context().comments();
                source.get_lines_before(comment.span, &comment_cursor)
            };

            if previous_comment.is_line() {
                write!(f, [hard_line_break()])?;
            } else if lines_before == 0 {
                write!(f, [space()])?;
            } else if lines_before == 1 {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [empty_line()])?;
            }
        }

        format_raw_comment(f, comment)?;
        previous_comment = Some(comment);
    }

    let last_comment = comments[comments.len() - 1];
    let right_span = f.context().span(right_id);
    let gap_span = Span::new(
        last_comment.span.file,
        last_comment.span.end,
        right_span.start,
    );

    if last_comment.is_line() || f.context().has_newline(gap_span) {
        if f.context().has_blank_line(gap_span) {
            write!(f, [empty_line()])?;
        } else {
            write!(f, [hard_line_break()])?;
        }
    } else {
        write!(f, [space()])?;
    }

    Ok(())
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
                let right_raw_prefix_comments = f.context().raw_prefix_comments_for(*right);
                let right_has_leading_prefix_comment =
                    expression_has_leading_prefix_comment(f.context(), *right);
                let right_has_raw_prefix_comments = !right_raw_prefix_comments.is_empty();
                let right_has_line_raw_prefix_comments = right_raw_prefix_comments
                    .iter()
                    .any(|comment| comment.is_line());
                let right_is_parenthesized =
                    matches!(tree.get(*right), Expression::Parenthesized { .. });
                let right_needs_comment_grouping = !right_is_parenthesized
                    && (right_has_leading_prefix_comment || right_has_raw_prefix_comments);
                let right_needs_inline_grouping = right_needs_await_or_yield_grouping;
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                let format_grouped_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if right_has_raw_prefix_comments {
                        write_grouped_prefix_unary_operand_comments(
                            f,
                            &right_raw_prefix_comments,
                            *right,
                        )?;
                        let right_comment_end = right_raw_prefix_comments
                            .last()
                            .map_or(f.context().span(*right).start, |comment| comment.span.end);

                        return write_expression_with_prefix_annotations_after_offset(
                            f,
                            *right,
                            right_comment_end,
                        );
                    }

                    write!(f, [right])
                });
                let can_inline_grouped_right =
                    right_has_raw_prefix_comments && !right_has_line_raw_prefix_comments;

                if needs_space {
                    if right_needs_comment_grouping {
                        if can_inline_grouped_right {
                            write!(
                                f,
                                [
                                    operator,
                                    space(),
                                    token("("),
                                    format_grouped_right,
                                    token(")")
                                ]
                            )?;
                        } else {
                            write!(
                                f,
                                [
                                    operator,
                                    space(),
                                    token("("),
                                    soft_block_indent(&format_grouped_right),
                                    token(")")
                                ]
                            )?;
                        }
                    } else if right_needs_inline_grouping {
                        write!(f, [operator, space(), token("("), right, token(")")])?;
                    } else {
                        write!(f, [operator, space(), right])?;
                    }
                } else if right_needs_comment_grouping {
                    if can_inline_grouped_right {
                        write!(f, [operator, token("("), format_grouped_right, token(")")])?;
                    } else {
                        write!(
                            f,
                            [
                                operator,
                                token("("),
                                soft_block_indent(&format_grouped_right),
                                token(")")
                            ]
                        )?;
                    }
                } else if right_needs_inline_grouping {
                    write!(f, [operator, token("("), right, token(")")])?;
                } else {
                    write!(f, [operator, right])?;
                }
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type shell
        Expression::Type { value } => {
            write!(f, [value])?;
        }

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
        Expression::Instantiation { left, .. } => {
            if instantiation_should_route_to_chain(f.context(), node_id, *left) {
                format_expression_chain(f, node_id)?;
            } else {
                format_instantiation_expression(f, node_id)?;
            }
        }

        // new
        Expression::New {
            left,
            generic_arguments,
            arguments,
        } => {
            format_new_expression(f, node_id, *left, generic_arguments, arguments)?;
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

        // assertions
        Expression::As {
            expression,
            target_type,
        } => {
            format_as_expression(f, node_id, *expression, *target_type)?;
        }
        Expression::Satisfies {
            expression,
            target_type,
        } => {
            format_satisfies_expression(f, node_id, *expression, *target_type)?;
        }
        Expression::Is { value, target_type } => {
            write!(f, [value, space(), token("is"), space(), target_type])?;
        }
        Expression::InstanceOf { value, target } => {
            write!(f, [value, space(), token("instanceof"), space(), target])?;
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

        // missing: preserve the surrounding source hole
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
        left, arguments, ..
    } = f.context().tree.get(node_id)
    else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for call-chain routing",
        });
    };

    let should_route_to_chain = call_should_route_to_chain(f.context(), node_id, *left, arguments);
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
