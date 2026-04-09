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
use crate::format::operator::types::{
    expression_uses_angle_assertion_syntax, format_type_binary_expression,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Comment, CommentKind, Expression, LocalNodeId, Mutability, PostfixPosition, TypeUnaryOperator,
    UnaryOperator,
};
use destack_fir::format::{Buffer, Format, FormatResult};
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

/// Collect boundary comments between one prefix unary operator and its operand.
fn prefix_unary_boundary_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let node_span = context.span(node_id);
    let right_span = context.span(right_id);
    if node_span.file != right_span.file || right_span.start <= node_span.start {
        return Vec::new();
    }

    let comments = context.comments();

    comments
        .comments_in_range(node_span.start, right_span.start)
        .to_vec()
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
                let boundary_comments =
                    prefix_unary_boundary_comments(f.context(), node_id, *right);
                let boundary_comments_require_multiline =
                    boundary_comments.iter().any(|comment| comment.is_line());
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
                    && (right_has_leading_prefix_comment
                        || right_has_raw_prefix_comments
                        || !boundary_comments.is_empty());
                let right_needs_inline_grouping = right_needs_await_or_yield_grouping;
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                let format_grouped_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if !boundary_comments.is_empty() {
                        write_grouped_prefix_unary_operand_comments(f, &boundary_comments, *right)?;
                        let boundary_end = boundary_comments
                            .last()
                            .map_or(f.context().span(*right).start, |comment| comment.span.end);

                        return write_expression_with_prefix_annotations_after_offset(
                            f,
                            *right,
                            boundary_end,
                        );
                    }

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
                let can_inline_grouped_right = (!boundary_comments.is_empty()
                    && !boundary_comments_require_multiline)
                    || (right_has_raw_prefix_comments && !right_has_line_raw_prefix_comments);

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
