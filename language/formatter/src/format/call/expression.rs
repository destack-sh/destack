use super::arguments::format_call_arguments;
use crate::format::annotation::format_trailing_comment_slice;
use crate::format::chain::extract_parenthesized_index_chain;
use crate::format::context::ParenthesizedExpressionView;
use crate::format::expression::{
    format_expression, format_generic_argument_list, should_unwrap_parenthesized_member_object,
    write_expression_without_trailing_annotations,
};
use crate::format::file::node_has_ignore_directive;
use crate::format::operator::expression_has_generic_arguments;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Argument, Expression, GenericArgument, LocalNodeId, NodeType, PostfixPosition};
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::prelude::{format_with, group, space, token};
use destack_fir::write;

/// Decide whether a call can drop one parenthesized callee wrapper.
pub(crate) fn call_drops_parenthesized_callee_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
) -> bool {
    matches!(
        parent_expression,
        Expression::Call { left, .. } if *left == parenthesized_id
    ) && expression_has_generic_arguments(context, inner_expression_id)
        && !context.has_annotation(parenthesized_id)
        && !context.has_annotation(inner_expression_id)
        && !ParenthesizedExpressionView::from_node(context, parenthesized_id)
            .is_some_and(ParenthesizedExpressionView::has_leading_inner_trivia)
}

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Call {
        position,
        left,
        generic_arguments,
        arguments,
    } = f.context().tree.get(node_id)
    {
        let callee_id = call_callee_expression_id(f.context(), *left);

        let format_inner = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let callee_span_end = f.context().span(callee_id).end;
            let optional_boundary = call_optional_boundary_position(f.context(), *left);
            let call_parent_is_decorator = f
                .context()
                .parent(node_id)
                .is_some_and(|(_, parent_type)| parent_type == NodeType::Decorator);

            // decorator parents own the callee formatting directly
            if call_parent_is_decorator {
                let callee_expression = f.context().tree.get(callee_id);
                let is_ignored = node_has_ignore_directive(f.context(), callee_id);
                format_expression(f, callee_id, callee_expression, is_ignored)?;
            }
            // generic arguments keep the callee trailing comments with the callee
            else if !generic_arguments.is_empty() {
                write!(f, [callee_id])?;
            }
            // otherwise the call expression owns trailing comments between callee and arguments
            else {
                write_expression_without_trailing_annotations(f, callee_id)?;

                let trailing_comments = if optional_boundary.is_some() {
                    let comments = f.context().comments();
                    comments
                        .comments_before_character(callee_span_end, b'?')
                        .to_vec()
                } else if arguments.is_empty() {
                    f.context()
                        .raw_comments_before_next_non_trivia_token_after_span(
                            f.context().span(callee_id),
                        )
                } else {
                    Vec::new()
                };

                if !trailing_comments.is_empty() {
                    write!(f, [format_trailing_comment_slice(&trailing_comments)])?;
                }
            }

            if let Some(optional_boundary) = optional_boundary {
                match optional_boundary {
                    PostfixPosition::Direct => write!(f, [token("?")])?,
                    PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
                }
            }

            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }

            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
            }

            format_call_arguments(f, node_id, arguments)
        });

        if matches!(f.context().tree.get(callee_id), Expression::Call { .. }) {
            write!(f, [group(&format_inner)])?;
        } else {
            write!(f, [format_inner])?;
        }
    } else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for call formatter",
        });
    }

    Ok(())
}

/// Return the callee expression without one optional-call wrapper.
fn call_callee_expression_id(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let Expression::Maybe { left, .. } = context.tree.get(left_id) else {
        return left_id;
    };

    *left
}

/// Return the optional boundary owned by one call callee wrapper.
fn call_optional_boundary_position(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> Option<PostfixPosition> {
    let Expression::Maybe { position, .. } = context.tree.get(left_id) else {
        return None;
    };

    Some(*position)
}

/// Format an instantiation expression.
#[inline]
pub(crate) fn format_instantiation_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Instantiation {
        left,
        generic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        format_generic_argument_list(f, generic_arguments)?;
    } else {
        debug_assert!(
            false,
            "unexpected expression kind for instantiation formatter"
        );
    }

    Ok(())
}

/// Format a `new` expression.
pub(crate) fn format_new_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if let Some((base_expression, indices)) =
        extract_parenthesized_index_chain(f.context().tree, left)
    {
        let formatted_left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token("("), base_expression])?;

            for index in &indices {
                write!(f, [token("["), *index, token("]")])?;
            }

            write!(f, [token(")")])
        });

        write!(f, [token("new"), space(), formatted_left])?;
    } else {
        let left = normalized_new_callee(f.context(), left);
        write!(f, [token("new"), space(), left])?;
    }

    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    format_call_arguments(f, node_id, arguments)?;

    Ok(())
}

/// Normalize one `new` callee by dropping one redundant parenthesized member wrapper.
fn normalized_new_callee(
    context: &DestackFormatContext<'_>,
    callee_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let Expression::Parenthesized { expression } = context.tree.get(callee_id) else {
        return callee_id;
    };

    if matches!(
        context.tree.get(*expression),
        Expression::Member { .. } | Expression::PrivateMember { .. }
    ) && should_unwrap_parenthesized_member_object(context, callee_id, *expression)
    {
        return *expression;
    }

    callee_id
}
