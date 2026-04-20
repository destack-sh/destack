use super::grouped::{
    arguments_grouped_layout, is_function_composition_args, write_grouped_arguments,
};
use super::list::{
    arguments_have_empty_line, call_arguments_have_ignored_ranges, format_all_args_broken_out,
    format_default_call_argument_list, format_long_curried_call_arguments,
    write_empty_call_arguments, write_ignored_call_arguments, write_simple_call_argument_list,
};
use super::pattern::{
    argument_is_interpolated_template_literal, argument_is_template_literal,
    call_uses_simple_list_layout, expression_is_long_curried_call,
};
use crate::format::annotation::{infix_or_postfix_annotations, prefix_annotations};
use crate::format::tree::has_multiline_jsx_argument;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Argument, DecoratorPosition, Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token};
use destack_fir::write;

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
            write!(f, [*name, token(":"), space(), *value])?;
        }
        Argument::Labeled { label, value, .. } => {
            write!(f, [*label, token(":"), space(), *value])?;
        }
        Argument::Positional { value, .. } => {
            write!(f, [*value])?;
        }
        Argument::Spread {
            label: Some(label),
            value,
            ..
        } => {
            write!(f, [token("..."), *label, token(":"), space(), *value])?;
        }
        Argument::Spread {
            label: None, value, ..
        } => {
            write!(f, [token("..."), *value])?;
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
        write_call_argument_node_body(f, node_id)
    }
}

/// Write one argument node without list-level trailing comment handling.
pub(crate) fn write_call_argument_node_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let argument = f.context().tree.get(node_id);

    if argument_is_plain_call_argument(f.context(), node_id) {
        write_plain_call_argument(f, node_id)?;
        return Ok(());
    }

    if argument_has_prefix_annotation(f.context(), node_id) {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    write_argument_with_value(argument, f)?;

    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

    Ok(())
}

/// Write one argument with its value payload.
fn write_argument_with_value<'ast>(
    argument: &Argument,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match argument {
        Argument::Named { name, value } => {
            write!(f, [name])?;
            write!(f, [token(":"), space()])?;
            write!(f, [*value])?;
        }
        Argument::Labeled { label, value } => {
            write!(f, [label])?;
            write!(f, [token(":"), space()])?;
            write!(f, [*value])?;
        }
        Argument::Positional { value } => {
            write!(f, [*value])?;
        }
        Argument::Spread { label, value } => {
            write!(f, [token("...")])?;

            if let Some(label) = label {
                write!(f, [label])?;
                write!(f, [token(":"), space()])?;
                write!(f, [*value])?;
            } else {
                write!(f, [*value])?;
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

/// Format call arguments when the surrounding context selects the layout policy.
pub(crate) fn format_call_arguments_in_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    format_call_arguments_impl(f, call_node_id, arguments, false)
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
    if call_arguments_have_ignored_ranges(f.context(), arguments) {
        return write_ignored_call_arguments(f, arguments, group_id);
    }

    // direct-list special cases
    if call_uses_simple_list_layout(f.context(), call_node_id, left, arguments) {
        return write_simple_call_argument_list(f, call_span, arguments);
    }

    // preserve intentional empty lines between arguments
    if arguments_have_empty_line(f.context(), arguments) {
        return format_all_args_broken_out(
            f,
            call_span,
            arguments,
            group_id,
            disallow_trailing_separator,
        );
    }

    // function composition
    if is_function_composition_args(f.context(), arguments) {
        return format_all_args_broken_out(
            f,
            call_span,
            arguments,
            group_id,
            disallow_trailing_separator,
        );
    }

    // grouped standard layouts
    if let Some(layout) = arguments_grouped_layout(f.context(), call_node_id, arguments) {
        return write_grouped_arguments(
            f,
            call_span,
            arguments,
            layout,
            group_id,
            disallow_trailing_separator,
        );
    }

    // long curried calls
    if allow_long_curried_layout && expression_is_long_curried_call(f.context(), call_node_id) {
        return format_long_curried_call_arguments(f, call_span, arguments);
    }

    // default layout
    let force_expand = has_multiline_jsx_argument(f.context(), arguments);
    format_default_call_argument_list(
        f,
        call_span,
        group_id,
        arguments,
        force_expand,
        disallow_trailing_separator,
    )
}
