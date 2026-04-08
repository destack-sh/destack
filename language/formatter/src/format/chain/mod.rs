mod argument;
mod expression;
mod groups;
mod member;
mod simple_argument;

pub(crate) use self::argument::{
    argument_value_id_if_present, is_lambda_expression, is_nested_lambda_expression,
    static_argument_list_is_hug_safe,
};
pub(crate) use self::expression::{
    MemberChain, format_expression_chain, is_assignment_chain_tail_lambda, is_call_like_argument,
};
pub(crate) use self::groups::{TailChainGroups, chain_instantiation_prefix_wrap_body_ops};
pub(crate) use self::member::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, assignment_like_parent,
    chain_base_trailing_node_id, chain_has_call_like_expression, chain_nodes,
    chain_operation_is_call_like, chain_operation_is_index, chain_operation_node_id,
    expression_has_ternary_ancestor, expression_trivia_anchor_end,
    extract_parenthesized_index_chain, first_tail_group_operation, format_maybe_expression,
    has_comment_between_expressions, is_chain_root, is_expression_chain, is_numeric_index,
    member_has_intervening_comment, member_is_private_hash, member_property_start,
    transparent_inner_expression,
};
pub(crate) use self::simple_argument::{expression_is_simple, expression_is_simple_with_depth};
