use super::argument::format_call_arguments;
use crate::annotation::FormatTrailingComments;
use crate::context::with_following_span_start;
use crate::expression::{
    format_expression, format_generic_argument_list, write_expression_without_trailing_comments,
};
use crate::file::node_has_ignore_directive;
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{Argument, Expression, GenericArgument, LocalNodeId, NodeType, PostfixPosition};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::{format_with, group, space, token};
use tspp_fir::write;

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let Expression::Call {
        position,
        left,
        generic_arguments,
        arguments,
        is_optional,
    } = f.context().tree.get(node_id)
    else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for call formatter",
        });
    };

    let callee_id = call_callee_expression_id(f.context(), *left);

    let format_inner = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
        let callee_span_end = f.context().span(callee_id).end;
        let optional_marker = is_optional.then_some(*position);
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
            write_expression_without_trailing_comments(f, callee_id)?;

            let trailing_comments = if optional_marker.is_some() {
                let comments = f.context().comments();
                comments
                    .comments_before_character(callee_span_end, b'?')
                    .to_vec()
            } else if arguments.is_empty() {
                f.context()
                    .comments_before_next_token_after_span(f.context().span(callee_id))
            } else {
                Vec::new()
            };

            if !trailing_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
            }
        }

        if optional_marker.is_some() {
            write!(f, [token("?")])?;
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

    Ok(())
}

/// Return the callee expression without one optional-call wrapper.
fn call_callee_expression_id(
    context: &TsppFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let Expression::Maybe { left, .. } = context.tree.get(left_id) else {
        return left_id;
    };

    *left
}

/// Format an instantiation expression.
#[inline]
pub(crate) fn format_instantiation_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let Expression::Instantiation {
        left,
        generic_arguments,
    } = f.context().tree.get(node_id)
    else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for instantiation formatter",
        });
    };

    write!(f, [*left])?;
    format_generic_argument_list(f, generic_arguments)?;

    Ok(())
}

/// Format a `new` expression.
pub(crate) fn format_new_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("new"), space()])?;

    let callee_following_span_start = arguments
        .first()
        .map(|argument_id| f.context().span(*argument_id).start)
        .or(f.context().following_span_start());
    with_following_span_start(f, callee_following_span_start, |f| write!(f, [left]))?;
    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    format_call_arguments(f, node_id, arguments)?;

    Ok(())
}
