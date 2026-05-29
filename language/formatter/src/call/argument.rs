use super::grouped::{
    arguments_grouped_layout, is_function_composition_args, write_grouped_arguments,
};
use super::list::{
    arguments_have_empty_line, call_arguments_have_ignored_ranges, format_all_args_broken_out,
    format_default_call_argument_list, format_long_curried_call_arguments,
    write_empty_call_arguments, write_ignored_call_arguments, write_simple_call_argument_list,
};
use super::pattern::{
    argument_expression_id, argument_is_interpolated_template_literal,
    argument_is_template_literal, call_uses_simple_list_layout, expression_is_long_curried_call,
};
use crate::annotation::{
    format_trailing_comments, infix_or_postfix_annotations, prefix_annotations,
};
use crate::expression::write_expression_without_trailing_comments;
use crate::tree::has_multiline_jsx_argument;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_dir::{
    Argument, Declaration, DecoratorPosition, Expression, LocalNodeId, NodeType, TypeExpression,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token};
use destack_fir::write;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    !context.has_annotation(argument_id)
        && matches!(
            context.tree.get(argument_id),
            Argument::Named { .. }
                | Argument::Labeled { .. }
                | Argument::Positional { .. }
                | Argument::Spread { .. }
        )
}

/// Write one call argument that is known to be plain.
pub(crate) fn write_plain_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [prefix_annotations(f.context(), argument_id)])?;

    match f.context().tree.get(argument_id) {
        Argument::Named { name, value, .. } => {
            write!(f, [*name, token(":"), space()])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Labeled { label, value, .. } => {
            write!(f, [*label, token(":"), space()])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Positional { value, .. } => {
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Spread {
            label: Some(label),
            value,
            ..
        } => {
            write!(f, [token("..."), *label, token(":"), space()])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Spread {
            label: None, value, ..
        } => {
            write!(f, [token("...")])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    write!(f, [infix_or_postfix_annotations(f.context(), argument_id)])?;

    Ok(())
}

/// Return whether an argument has a prefix annotation.
pub(crate) fn argument_has_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .annotation_ids(argument_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let position = context.annotation(annotation_id).position;

            matches!(
                position,
                DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
            )
        })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // non call-like argument nodes still use the local body-only path
        if !argument_uses_call_node_comments(f.context(), node_id) {
            return write_call_argument_node_body(f, node_id);
        }

        let trailing_span = argument_trailing_span(f.context(), node_id);
        let enclosing_span = argument_enclosing_span(f.context(), node_id);
        let following_span_start = f.context().following_span_start();

        // leading comments and annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // payload
        write_call_argument_payload(self, f)?;

        // trailing comments
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                trailing_span,
                following_span_start,
            )]
        )?;

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

/// Return whether one argument node is in one call-like parent that uses generic node comments.
fn argument_uses_call_node_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    match parent_type {
        NodeType::Expression => matches!(
            context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
            Expression::Call { .. }
                | Expression::New { .. }
                | Expression::NewMaybe { .. }
                | Expression::Import { .. }
        ),
        NodeType::TypeExpression => false,
        _ => false,
    }
}

/// Return the enclosing span used for one argument node's trailing comments.
pub(crate) fn argument_enclosing_span(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Argument>,
) -> Span {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        unreachable!("argument node should have one parent");
    };

    match parent_type {
        NodeType::Expression => context.span(LocalNodeId::<Expression>::new(parent_id)),
        NodeType::TypeExpression => context.span(LocalNodeId::<TypeExpression>::new(parent_id)),
        _ => unreachable!("call-like argument node parent should be one expression"),
    }
}

/// Return the span that owns trailing comments for one argument node.
pub(crate) fn argument_trailing_span(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Argument>,
) -> Span {
    let node_span = context.span(node_id);
    let Some(value_id) = argument_expression_id(context, node_id) else {
        return node_span;
    };
    let value_span = argument_value_trailing_span(context, value_id);

    if value_span.end <= node_span.end {
        return node_span;
    }

    Span::new(node_span.file, node_span.start, value_span.end)
}

/// Return the trailing-comment anchor span for one argument value.
fn argument_value_trailing_span(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> Span {
    let value_span = context.span(value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return value_span;
    };
    let Declaration::Function(function) = context.tree.get(*declaration_id) else {
        return value_span;
    };

    if let Some(body_id) = function.body
        && let Some(body_span) = context
            .tree
            .get_side_span(*declaration_id, NodeSpanType::Region(NodeSpanRegion::Body))
    {
        let start = value_span.start;
        let end = body_span.end.max(context.span(body_id).end);

        return Span::new(value_span.file, start, end);
    }

    if let Some(parameter_span) = context.tree.get_side_span(
        *declaration_id,
        NodeSpanType::Region(NodeSpanRegion::Parameters),
    ) {
        return Span::new(value_span.file, value_span.start, parameter_span.end);
    }

    value_span
}

/// Format one node while exposing one following sibling start to trailing comment logic.
pub(crate) fn with_argument_following_span_start<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    following_span_start: u32,
    content: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
) -> FormatResult<()> {
    // install
    let previous_following_span_start = f
        .context_mut()
        .replace_following_span_start(following_span_start);

    // format
    let result = content(f);

    // restore
    f.context_mut()
        .replace_following_span_start(previous_following_span_start);

    result
}

/// Format one argument while exposing one following sibling start to trailing comment logic.
pub(crate) fn write_argument_with_following_span_start<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    following_span_start: u32,
) -> FormatResult<()> {
    with_argument_following_span_start(f, following_span_start, |f| write!(f, [argument_id]))
}

/// Write one argument node without list-level trailing comment handling.
pub(crate) fn write_call_argument_node_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let argument = f.context().tree.get(node_id);

    // plain fast path
    if argument_is_plain_call_argument(f.context(), node_id) {
        write_plain_call_argument(f, node_id)?;
        return Ok(());
    }

    // leading prefix annotations
    if argument_has_prefix_annotation(f.context(), node_id) {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    // payload
    write_call_argument_payload(argument, f)?;

    // trailing annotations
    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

    Ok(())
}

/// Write one argument payload without node-local comment ownership.
fn write_call_argument_payload<'ast>(
    argument: &Argument,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match argument {
        Argument::Named { name, value } => {
            write!(f, [name])?;
            write!(f, [token(":"), space()])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Labeled { label, value } => {
            write!(f, [label])?;
            write!(f, [token(":"), space()])?;
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Positional { value } => {
            write_expression_without_trailing_comments(f, *value)?;
        }
        Argument::Spread { label, value } => {
            write!(f, [token("...")])?;
            if let Some(label) = label {
                write!(f, [label])?;
                write!(f, [token(":"), space()])?;
                write_expression_without_trailing_comments(f, *value)?;
            } else {
                write_expression_without_trailing_comments(f, *value)?;
            }
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    format_call_arguments_impl(f, call_node_id, arguments, true)
}

/// Format call arguments with explicit long-curried-call handling control.
fn format_call_arguments_impl<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
    allow_long_curried_layout: bool,
) -> FormatResult<()> {
    let group_id = f.group_id("call_args");
    let call_span = f.context().span(call_node_id);
    let left = match f.context().tree.get(call_node_id) {
        Expression::Call { left, .. } => *left,
        _ => call_node_id,
    };
    let disallow_trailing_separator = arguments.first().copied().is_some_and(|argument_id| {
        argument_is_template_literal(f.context(), argument_id)
            && !argument_is_interpolated_template_literal(f.context(), argument_id)
    });

    // empty list
    if arguments.is_empty() {
        return write_empty_call_arguments(f, call_node_id);
    }

    // ignored ranges
    let result = if call_arguments_have_ignored_ranges(f.context(), arguments) {
        write_ignored_call_arguments(f, arguments, group_id)
    }
    // direct-list special cases
    else if call_uses_simple_list_layout(f.context(), call_node_id, left, arguments) {
        write_simple_call_argument_list(f, call_span, arguments)
    }
    // force expanded layouts
    else if arguments_have_empty_line(f.context(), arguments)
        || is_function_composition_args(f.context(), arguments)
    {
        format_all_args_broken_out(
            f,
            call_span,
            arguments,
            group_id,
            disallow_trailing_separator,
        )
    }
    // grouped standard layouts
    else if let Some(layout) = arguments_grouped_layout(f.context(), call_node_id, arguments) {
        write_grouped_arguments(f, arguments, layout, group_id, disallow_trailing_separator)
    }
    // long curried calls
    else if allow_long_curried_layout
        && expression_is_long_curried_call(f.context(), call_node_id)
    {
        format_long_curried_call_arguments(f, call_span, arguments)
    }
    // default layout
    else {
        let force_expand = has_multiline_jsx_argument(f.context(), arguments);
        format_default_call_argument_list(
            f,
            call_span,
            group_id,
            arguments,
            force_expand,
            disallow_trailing_separator,
        )
    };

    result?;

    f.context_mut()
        .comments_mut()
        .skip_comments_before(call_span.end);

    Ok(())
}
