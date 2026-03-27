mod analysis;
mod base;
mod r#break;
mod format;
mod normalize;

pub(crate) use self::analysis::{
    argument_value_id_if_present, chain_has_parent_intervening_break_or_comment, chain_head_id,
    chain_member_has_promotable_boundary_comment, expression_trivia_anchor_end,
    has_comment_between_expressions, has_line_comment_between_expressions, is_lambda_expression,
    is_nested_lambda_expression, is_numeric_index, is_simple_chain_static_arguments,
    lambda_expression_should_break, member_has_intervening_comment, member_is_private_hash,
    path_postfix_annotations_emit_on_tail, should_expand_static_argument_list,
    should_force_multiline_mapped_type, static_argument_list_is_hug_safe,
};
pub(crate) use self::base::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, assignment_like_parent,
    chain_base_trailing_node_id, chain_expression_from_node, chain_node_left_id, chain_nodes,
    chain_operation_is_index, chain_operation_node_id, extract_parenthesized_index_chain,
    first_grouped_line_operation, format_maybe_expression, has_chain_parent, is_chain_root,
    is_expression_chain, should_use_trailing_coalesce, transparent_inner_expression,
};
pub(crate) use self::r#break::{
    call_has_parenthesized_await_member_receiver, chain_has_breaking_annotations,
    chain_has_intervening_comment, chain_has_optional_call_boundary_trivia,
    chain_line_starts_with_block_prefix_annotation, chain_node_has_breaking_annotation,
    chain_node_has_non_inline_annotation, chain_overflows_in_type_binary_left,
    expression_is_in_conditional_branch, is_assignment_chain_tail_lambda,
    receiver_is_await_wrapped, should_split_chain_root_path_segments,
};
pub(crate) use self::format::{format_expression_chain, should_parenthesize_index_expression};
