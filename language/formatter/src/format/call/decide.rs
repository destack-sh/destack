use crate::analysis::timing::tags;
use crate::analysis::{
    CallArgumentCommentProfile, CallArgumentLayoutDecision, CallArgumentPlannerState,
    HugLastCallArgumentLayout, SingleSimpleArgumentInlineOptions,
    argument_has_line_comment_annotation, argument_is_collection_literal,
    argument_is_interpolated_template_literal, call_arguments_use_single_callback_argument_inline,
    call_arguments_use_single_simple_argument_inline,
    call_has_leading_block_callback_with_simple_tail, can_consider_hug_last_call_arguments,
    collect_call_argument_comment_profile, resolve_hug_last_call_argument_layout,
    resolve_inline_call_width_hint_without_static_arguments,
    resolve_regular_call_argument_expansion_profile,
};
use crate::expression::{
    Argument, DestackFormatContext, Expression, LocalNodeId, argument_is_template_literal,
    argument_value_id,
};

/// Decide default list layout after layout checks.
fn list_default_call_argument_plan(
    force_expand: bool,
    has_line_comment_annotations: bool,
) -> CallArgumentLayoutDecision {
    CallArgumentLayoutDecision::ListDefault {
        force_expand,
        has_line_comment_annotations,
    }
}

/// Return whether comment profiling forces explicit multiline expansion.
fn call_argument_comments_force_expanded_layout(
    dynamic_arguments: &[LocalNodeId<Argument>],
    comment_profile: &CallArgumentCommentProfile,
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return false;
    }

    comment_profile.has_line_comment_annotations
        || comment_profile.has_prefix_line_comment_annotations
}

/// Return whether trailing collection comments should force list expansion.
fn call_argument_trailing_collection_comment_force_expand(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
    has_line_comment_annotations: bool,
) -> bool {
    if dynamic_arguments.len() <= 1 || !trailing_collection_argument {
        return false;
    }

    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    let has_last_line_comment_annotation = has_line_comment_annotations
        && argument_has_line_comment_annotation(context, last_argument_id);
    let last_argument_span = context.span(last_argument_id);
    let has_last_source_comment = context.has_comment(last_argument_span);

    has_last_line_comment_annotation || has_last_source_comment
}

/// Return whether default list rendering should use trailing collection expansion.
fn call_arguments_use_trailing_collection_expanded_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    force_expand: bool,
    has_any_argument_annotation: bool,
    has_line_comment_annotations: bool,
    has_boundary_comments: bool,
) -> bool {
    let inline_width_hint = planner_state
        .inline_call_width_hint_without_static_arguments
        .or_else(|| {
            if planner_state.call_has_static_arguments {
                return None;
            }

            resolve_inline_call_width_hint_without_static_arguments(
                context,
                call_node_id,
                planner_state.call_has_static_arguments,
            )
        });
    let exceeds_line_width = inline_width_hint
        .is_some_and(|inline_width_hint| inline_width_hint > planner_state.line_width);
    let should_expand_trailing_collection = force_expand || exceeds_line_width;
    let has_trailing_collection_argument = dynamic_arguments
        .last()
        .copied()
        .is_some_and(|argument_id| argument_is_collection_literal(context, argument_id));
    if !should_expand_trailing_collection
        || dynamic_arguments.len() <= 1
        || !has_trailing_collection_argument
        || has_any_argument_annotation
        || has_line_comment_annotations
        || has_boundary_comments
    {
        return false;
    }

    !trailing_collection_argument_has_comment_signal(context, dynamic_arguments)
}

/// Return whether the trailing collection argument has non blank comment signals.
pub(super) fn trailing_collection_argument_has_comment_signal(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    if !argument_is_collection_literal(context, last_argument_id) {
        return false;
    }

    if context.has_non_blank_annotation(last_argument_id) {
        return true;
    }

    let last_argument_value_id = argument_value_id(context.tree, last_argument_id);
    context.has_non_blank_annotation(last_argument_value_id)
}
/// Build call argument comment profile only when annotations require it.
fn resolve_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> CallArgumentCommentProfile {
    if has_any_argument_annotation || has_call_infix_annotations {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE);
        context.increment_counter("call.arguments.comment_profile.calls", 1);
        return collect_call_argument_comment_profile(context, dynamic_arguments);
    }

    context.increment_counter("call.arguments.comment_profile.skip_no_annotation", 1);
    CallArgumentCommentProfile::default()
}

/// Return whether one single template literal argument can stay inline.
fn call_arguments_use_single_template_literal_argument_inline(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    has_boundary_comments: bool,
) -> bool {
    if dynamic_arguments.len() != 1
        || has_boundary_comments
        || planner_state.has_call_infix_annotations
        || planner_state.argument_shape.has_any_argument_annotation
    {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    argument_is_template_literal(context, argument_id)
        && !argument_is_interpolated_template_literal(context, argument_id)
}

/// Decide post-hugged call argument layout.
pub(super) fn decide_post_hugged_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    has_boundary_comments: bool,
) -> CallArgumentLayoutDecision {
    // keep single callback arguments wrapped directly to avoid list-style trailing commas
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        context,
        dynamic_arguments,
        planner_state.has_call_infix_annotations,
        planner_state.force_expand_single_long_with_static_arguments,
        planner_state.force_expand_single_collection_for_type_binary_callee,
    ) && !has_boundary_comments;
    if use_single_callback_argument_inline {
        context.increment_counter("call.arguments.path.single_callback_inline", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep short single positional arguments inline
    let use_single_simple_argument = call_arguments_use_single_simple_argument_inline(
        context,
        dynamic_arguments,
        SingleSimpleArgumentInlineOptions {
            line_width: planner_state.line_width,
            call_has_static_arguments: planner_state.call_has_static_arguments,
            force_expand_single_long_with_static_arguments: planner_state
                .force_expand_single_long_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee: planner_state
                .force_expand_single_collection_for_type_binary_callee,
            single_argument_force_expand: planner_state.single_argument_force_expand,
        },
        planner_state.argument_shape,
        planner_state.inline_call_width_hint_without_static_arguments,
    );
    if use_single_simple_argument {
        context.increment_counter("call.arguments.path.single_simple", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep non-interpolated template literal snapshot arguments wrapped inline
    if call_arguments_use_single_template_literal_argument_inline(
        context,
        dynamic_arguments,
        planner_state,
        has_boundary_comments,
    ) {
        context.increment_counter("call.arguments.path.single_template_inline", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep leading callback plus short tail calls inline
    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let use_leading_block_callback_inline = has_leading_block_callback_with_simple_tail;
    if use_leading_block_callback_inline {
        context.increment_counter("call.arguments.path.leading_block_callback_inline", 1);
        return CallArgumentLayoutDecision::InlineAll;
    }

    // boundary comments should block aggressive inline and hug-last behavior
    let has_any_argument_annotation = planner_state.argument_shape.has_any_argument_annotation;

    // comments can force explicit multiline argument rendering
    let comment_profile = resolve_call_argument_comment_profile(
        context,
        dynamic_arguments,
        has_any_argument_annotation,
        planner_state.has_call_infix_annotations,
    );
    let has_line_comment_annotations = comment_profile.has_line_comment_annotations;
    if call_argument_comments_force_expanded_layout(dynamic_arguments, &comment_profile) {
        context.increment_counter("call.arguments.path.comment_expanded", 1);
        return CallArgumentLayoutDecision::CommentExpanded(comment_profile);
    }

    // expansion profile drives default and hug-last policy paths
    let expansion_profile = {
        let _timing =
            context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE);
        resolve_regular_call_argument_expansion_profile(context, call_node_id, dynamic_arguments)
    };
    let has_call_infix_annotations = expansion_profile.has_call_infix_annotations;
    let trailing_collection_argument = expansion_profile.trailing_collection_argument;
    let force_expand = expansion_profile.force_expand
        || has_boundary_comments
        || call_argument_trailing_collection_comment_force_expand(
            context,
            dynamic_arguments,
            trailing_collection_argument,
            has_line_comment_annotations,
        );

    // hug-last candidates can still end in default list rendering
    let can_consider_hug_last_argument = can_consider_hug_last_call_arguments(
        context,
        dynamic_arguments,
        has_line_comment_annotations || has_boundary_comments,
        has_call_infix_annotations,
    );
    if can_consider_hug_last_argument {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
        if let Some(layout) = resolve_hug_last_call_argument_layout(
            context,
            call_node_id,
            dynamic_arguments,
            planner_state,
            force_expand,
            trailing_collection_argument,
        ) {
            return match layout {
                HugLastCallArgumentLayout::Inline => CallArgumentLayoutDecision::InlineAll,
            };
        }
    }

    if call_arguments_use_trailing_collection_expanded_layout(
        context,
        call_node_id,
        dynamic_arguments,
        planner_state,
        force_expand,
        has_any_argument_annotation,
        has_line_comment_annotations,
        has_boundary_comments,
    ) {
        context.increment_counter("call.arguments.path.trailing_collection_expanded", 1);
        return CallArgumentLayoutDecision::TrailingCollectionExpanded;
    }

    context.increment_counter("call.arguments.path.list_default", 1);
    list_default_call_argument_plan(force_expand, has_line_comment_annotations)
}
