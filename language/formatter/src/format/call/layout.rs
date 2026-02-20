use super::callback::{
    single_callback_argument_prefers_trailing_separator_wrap,
    write_single_callback_argument_wrapped_with_trailing_separator,
};
use super::decide::decide_post_hugged_call_argument_layout;
use super::render::{render_call_argument_plan, write_single_call_argument_inline_wrapped};
use super::separator::{
    format_single_plain_argument_with_separator_line_comment,
    single_argument_separator_line_comment_source,
};
use crate::analysis::timing::tags;
use crate::analysis::{
    CallArgumentLayoutRenderState, CallArgumentPlannerBaseState, CallArgumentShape,
    SingleSimpleArgumentShortCircuitOptions, argument_has_callback_blocking_comment_annotation,
    argument_has_multiline_prefix_annotation, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, argument_is_plain_call_argument,
    build_call_argument_planner_base_state, build_call_argument_planner_state,
    call_argument_layout_facts_are_simple_multi_unannotated, call_arguments_have_boundary_comments,
    call_arguments_use_single_callback_argument_inline,
    call_arguments_use_single_simple_argument_short_circuit,
    call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, call_has_non_blank_infix_annotation,
    call_has_static_arguments, call_should_force_hugged_expand, resolve_call_argument_layout_facts,
    resolve_chain_call_argument_force_expand,
    resolve_inline_call_width_hint_without_static_arguments,
    single_argument_requires_expanded_list,
};
use crate::directive::any_ignore_range_for_nodes;
use crate::expression::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, GroupId, HugOptions, LocalNodeId, argument_is_function_expression,
    argument_is_lambda_expression, block_indent, format_hugged, list_like, token,
};
use destack_ast::{Comment, CommentStyle};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS);

    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);
    let result = format_call_arguments_with_group(f, call_node_id, dynamic_arguments, group_id);
    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if annotation.position() != AnnotationPosition::BlockInfix {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                if context.has_newline(annotation_span) {
                    return true;
                }

                match annotation {
                    Annotation::Comment { node, .. } => {
                        let comment = context.tree.get::<Comment>(node);
                        comment.style == CommentStyle::Slash
                    }
                    Annotation::Blank { .. } | Annotation::Doc { .. } => true,
                    Annotation::Decorator { .. } => false,
                }
            })
        })
        .unwrap_or(false)
}

/// Format call arguments with an active list group id.
fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let single_argument = [argument_id];
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, &single_argument);
    let separator_line_comment_source = if argument_is_plain_call_argument(f.context(), argument_id)
    {
        single_argument_separator_line_comment_source(f.context(), call_node_id, argument_id)
    } else {
        None
    };

    // shared single argument planner base state
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(f.context(), call_node_id);
    let argument_annotation_facts = f.context().ensure_argument_annotation_facts(argument_id);
    let has_any_argument_annotation = argument_annotation_facts.has_prefix_annotation
        || argument_annotation_facts.has_comment
        || argument_annotation_facts.has_line_comment
        || argument_annotation_facts.has_prefix_line_comment;
    let is_multiline_in_source = f.context().node_has_newline(argument_id);
    let argument_shape = CallArgumentShape {
        has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: !has_any_argument_annotation && !is_multiline_in_source,
    };
    let planner_base_state = CallArgumentPlannerBaseState {
        line_width: usize::from(f.context().options.line_width),
        call_has_static_arguments: call_has_static_arguments(f.context(), call_node_id),
        has_call_infix_annotations,
        argument_shape,
    };
    let single_argument_force_expand =
        single_argument_requires_expanded_list(f.context(), &single_argument);
    let inline_call_width_hint_without_static_arguments =
        if planner_base_state.call_has_static_arguments {
            None
        } else {
            resolve_inline_call_width_hint_without_static_arguments(
                f.context(),
                call_node_id,
                planner_base_state.call_has_static_arguments,
            )
        };

    // separator line comments after a single plain argument need comma before comment
    if let Some(comment_source) = separator_line_comment_source.as_ref() {
        format_single_plain_argument_with_separator_line_comment(f, argument_id, comment_source)?;
        return Ok(());
    }

    // keep very common single simple arguments wrapped inline
    let use_single_simple_argument_short_circuit =
        call_arguments_use_single_simple_argument_short_circuit(
            f.context(),
            &single_argument,
            SingleSimpleArgumentShortCircuitOptions {
                line_width: planner_base_state.line_width,
                call_has_static_arguments: planner_base_state.call_has_static_arguments,
                has_call_infix_annotations: planner_base_state.has_call_infix_annotations,
                single_argument_force_expand,
            },
            planner_base_state.argument_shape,
            inline_call_width_hint_without_static_arguments,
        ) && !has_boundary_comments;
    if use_single_simple_argument_short_circuit {
        f.context()
            .increment_counter("call.arguments.single_simple.short_circuit", 1);
        f.context()
            .increment_counter("call.arguments.path.single_simple.short_circuit", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // keep closure-cast object arguments inline in call-then-chain no-semi seams
    let use_inline_closure_cast_object_argument =
        !has_boundary_comments && argument_is_inline_closure_cast_object(f.context(), argument_id);
    if use_inline_closure_cast_object_argument {
        f.context()
            .increment_counter("call.arguments.path.inline_closure_cast_object", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // preserve inline callback single argument rendering
    let force_expand_single_multiline_with_static_arguments =
        call_force_expand_single_multiline_with_static_arguments(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        f.context(),
        &single_argument,
        planner_base_state.has_call_infix_annotations,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
    ) && !has_boundary_comments;
    if use_single_callback_argument_inline {
        let use_trailing_separator_wrap = single_callback_argument_prefers_trailing_separator_wrap(
            f.context(),
            call_node_id,
            argument_id,
        );
        f.context()
            .increment_counter("call.arguments.path.single_callback_inline", 1);
        if use_trailing_separator_wrap {
            f.context().increment_counter(
                "call.arguments.path.single_callback_inline.trailing_wrap",
                1,
            );
            write_single_callback_argument_wrapped_with_trailing_separator(f, argument_id)?;
        } else {
            write_single_call_argument_inline_wrapped(f, argument_id)?;
        }
        return Ok(());
    }

    // keep single collection arguments on hugged path when allowed
    let force_hugged_expand =
        call_should_force_hugged_expand(force_expand_single_collection_for_type_binary_callee);
    let has_hug_blocking_comment_annotation =
        argument_has_callback_blocking_comment_annotation(f.context(), argument_id);
    let can_use_hugged = !has_boundary_comments
        && !has_any_argument_annotation
        && !argument_has_multiline_prefix_annotation(f.context(), argument_id)
        && !has_hug_blocking_comment_annotation
        && !planner_base_state.has_call_infix_annotations
        && !force_expand_single_multiline_with_static_arguments
        && !argument_is_lambda_expression(f.context(), argument_id)
        && !argument_is_function_expression(f.context(), argument_id)
        && !argument_is_interpolated_template_literal(f.context(), argument_id);
    if can_use_hugged {
        let used_hugged = format_hugged(
            f,
            &single_argument,
            HugOptions::CALL,
            Some(group_id),
            force_hugged_expand,
        )?;
        if used_hugged {
            f.context()
                .increment_counter("call.arguments.path.hugged", 1);
            return Ok(());
        }
    }

    // run profiled single argument planner decision
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        &single_argument,
        planner_base_state,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        inline_call_width_hint_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let plan = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            &single_argument,
            planner_state,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        call_node_id,
        group_id,
        has_any_argument_annotation: planner_base_state
            .argument_shape
            .has_any_argument_annotation,
        all_plain_call_arguments: argument_is_plain_call_argument(f.context(), argument_id),
    };
    render_call_argument_plan(f, &single_argument, plan, render_state)
}

/// Format call arguments with an active list group id.
fn format_call_arguments_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    // empty argument lists can still carry boundary infix annotations
    if dynamic_arguments.is_empty() {
        if f.context().has_infix_annotation(call_node_id) {
            let should_expand_multiline =
                empty_call_infix_requires_multiline(f.context(), call_node_id);
            if should_expand_multiline {
                write!(
                    f,
                    [
                        token("("),
                        block_indent(&f.context().block_infix_annotations(call_node_id)),
                        token(")")
                    ]
                )?;
            } else {
                write!(
                    f,
                    [
                        token("("),
                        f.context().block_infix_annotations(call_node_id),
                        token(")")
                    ]
                )?;
            }
        } else {
            write!(f, [token("("), token(")")])?;
        }
        return Ok(());
    }

    // ignore ranges: route through list_like so raw span preservation stays consistent
    if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        let has_ignore_ranges =
            any_ignore_range_for_nodes(f.context(), dynamic_arguments, comment_tokens);
        if has_ignore_ranges {
            f.context()
                .increment_counter("call.arguments.path.ignore_ranges_list_like", 1);
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id)).force_expand();
            write!(f, [list])?;
            return Ok(());
        }
    }

    if dynamic_arguments.len() == 1 {
        return format_single_call_argument_with_group(
            f,
            call_node_id,
            dynamic_arguments[0],
            group_id,
        );
    }

    let layout_facts =
        resolve_call_argument_layout_facts(f.context(), call_node_id, dynamic_arguments);
    let planner_base_state =
        build_call_argument_planner_base_state(f.context(), call_node_id, layout_facts);
    let all_plain_call_arguments = layout_facts.all_plain_call_arguments;
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, dynamic_arguments);

    let inline_call_width_hint_without_static_arguments = None;
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        dynamic_arguments,
        planner_base_state,
        false,
        false,
        inline_call_width_hint_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let plan = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            dynamic_arguments,
            planner_state,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        call_node_id,
        group_id,
        has_any_argument_annotation: layout_facts.has_any_argument_annotation,
        all_plain_call_arguments,
    };
    render_call_argument_plan(f, dynamic_arguments, plan, render_state)
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = context.lookup_call_argument_chain_force_expand(call_node_id) {
        context.increment_counter("call.arguments.chain.fast_cache.hits", 1);
        context.increment_counter(
            if force_expand {
                "call.arguments.chain.force_expand.true"
            } else {
                "call.arguments.chain.force_expand.false"
            },
            1,
        );
        return force_expand;
    }
    context.increment_counter("call.arguments.chain.fast_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.fast_false.empty", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_facts = resolve_call_argument_layout_facts(context, call_node_id, dynamic_arguments);
    let should_bypass_simple_fast_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(context, dynamic_arguments);
    let can_use_simple_fast_false =
        call_argument_layout_facts_are_simple_multi_unannotated(layout_facts)
            && !should_bypass_simple_fast_false;
    if can_use_simple_fast_false {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.fast_false.simple", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand =
        resolve_chain_call_argument_force_expand(context, call_node_id, dynamic_arguments);
    context.store_call_argument_chain_force_expand(call_node_id, force_expand);
    force_expand
}
