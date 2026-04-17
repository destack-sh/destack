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
use crate::DestackFormatter;
use crate::format::tree::has_multiline_jsx_argument;
use destack_ast::{Argument, Expression, LocalNodeId};
use destack_fir::format::FormatResult;

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    format_call_arguments_impl(f, call_node_id, arguments, true)
}

/// Format call arguments when the surrounding owner selects the layout policy.
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

    // upstream direct-list special cases
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
