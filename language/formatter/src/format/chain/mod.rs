mod argument;
mod expression;
mod groups;
mod member;

pub(crate) use self::argument::{
    argument_value_id_if_present, is_lambda_expression, is_nested_lambda_expression,
    is_simple_chain_static_arguments, lambda_expression_should_break,
    should_expand_static_argument_list, static_argument_list_is_hug_safe,
};
pub(crate) use self::expression::{
    call_has_parenthesized_await_member_receiver, chain_annotation_is_inline_non_breaking,
    chain_node_has_forcing_annotation, chain_node_has_non_inline_annotation,
    chain_overflows_in_type_binary_left, format_expression_chain, is_assignment_chain_tail_lambda,
    is_call_like_argument, receiver_is_await_wrapped,
};
pub(crate) use self::groups::{chain_instantiation_prefix_wrap_body_ops, chain_should_break};
pub(crate) use self::member::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, assignment_like_parent,
    build_member_chain_parts, chain_base_trailing_node_id,
    chain_has_parent_intervening_break_or_comment, chain_head_id,
    chain_member_has_promotable_boundary_comment, chain_nodes, chain_operation_is_call_like,
    chain_operation_is_index, chain_operation_node_id, expression_has_ternary_ancestor,
    expression_trivia_anchor_end, extract_parenthesized_index_chain, first_grouped_line_operation,
    format_maybe_expression, has_chain_parent, has_comment_between_expressions,
    has_line_comment_between_expressions, is_chain_root, is_expression_chain, is_numeric_index,
    member_has_intervening_comment, member_is_private_hash, should_use_trailing_coalesce,
    transparent_inner_expression,
};
