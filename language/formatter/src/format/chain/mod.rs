mod argument;
mod expression;
mod groups;
mod member;
mod simple;

pub(crate) use self::argument::{
    argument_value_id_if_present, is_lambda_expression, is_nested_lambda_expression,
};
pub(crate) use self::expression::{
    MemberChain, format_expression_chain, is_assignment_chain_tail_lambda,
};
pub(crate) use self::groups::TailChainGroups;
pub(crate) use self::member::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, assignment_like_parent,
    chain_has_call_like_expression, chain_nodes, chain_operation_is_call_like,
    chain_operation_is_index, chain_operation_node_id, expression_has_ternary_ancestor,
    expression_trivia_anchor_end, first_tail_group_operation, format_maybe_expression,
    has_comment_between_expressions, is_expression_chain, is_numeric_index,
    member_has_intervening_comment, member_is_private_hash, member_property_start,
    transparent_inner_expression,
};
pub(crate) use self::simple::SimpleArgument;
