mod analyze;
mod call;
mod classify;
pub mod scan;
pub mod timing;

pub(crate) use analyze::{
    CallArgumentCommentProfile, argument_has_callback_blocking_comment_annotation,
    argument_is_plain_call_argument, call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, call_should_force_hugged_expand,
    collect_call_argument_comment_profile, resolve_chain_call_argument_force_expand,
    resolve_regular_call_argument_expansion_profile, single_argument_requires_expanded_list,
    write_inline_call_argument_list, write_plain_call_argument, write_plain_call_argument_or_node,
};
pub(crate) use call::{
    CallArgumentLayoutDecision, CallArgumentLayoutRenderState, CallArgumentPlannerBaseState,
    CallArgumentPlannerState, CallArgumentShape, HugLastCallArgumentLayout,
    SingleSimpleArgumentInlineOptions, SingleSimpleArgumentShortCircuitOptions,
    build_call_argument_planner_base_state, build_call_argument_planner_state,
    call_argument_layout_facts_are_simple_multi_unannotated,
    call_arguments_use_single_callback_argument_inline,
    call_arguments_use_single_simple_argument_inline,
    call_arguments_use_single_simple_argument_short_circuit, can_consider_hug_last_call_arguments,
    resolve_call_argument_layout_facts, resolve_hug_last_call_argument_layout,
    resolve_inline_call_width_hint_without_static_arguments,
};
pub(crate) use classify::{
    ArgumentSimplicityOptions, argument_has_line_comment_annotation,
    argument_has_multiline_prefix_annotation, argument_has_non_blank_annotation,
    argument_has_source_separator_line_comment_annotation, argument_is_collection_literal,
    argument_is_inline_closure_cast_object, argument_is_interpolated_template_literal,
    argument_is_simple_with_options, call_arguments_are_multiline_in_source,
    call_arguments_have_boundary_comments, call_arguments_preserve_blank_line_between,
    call_has_leading_block_callback_with_simple_tail, call_has_non_blank_infix_annotation,
    call_has_static_arguments, is_call_like_argument, is_simple_static_argument,
    is_tree_attribute_expression,
};
