mod assign;
mod binary;
mod expression;
mod postfix;
mod tokens;
mod r#type;
mod types;
mod union;

pub(crate) use self::assign::assignment_drops_parenthesized_operand_wrapper;
pub(crate) use self::binary::{
    binary_keeps_unary_left_parenthesized_wrapper, flattened_binary_operand_count,
    format_binary_expression,
};
pub(crate) use self::expression::{
    format_operator_expression, write_operator_expression_trailing_annotations,
};
pub(crate) use self::postfix::{
    is_chain_expression, needs_parens_in_postfix_position, write_postfix_base_expression,
};
pub(crate) use self::r#type::{
    expression_has_static_type_arguments, expression_has_type_grouping_semantics,
    expression_static_arguments, format_static_argument_list,
    format_static_argument_list_with_relational_spacing, leading_raw_type_position_comment_nodes,
    normalize_parenthesized_type_grouping_inner_expression,
    parenthesized_type_expression_prefers_soft_block_layout,
    raw_type_position_comment_nodes_in_range, should_drop_parenthesized_type_expression,
    write_colon_prefixed_type_annotation, write_expression_with_inline_prefix_annotations,
    write_type_expression_with_inline_prefix_annotations,
    write_type_expression_without_prefix_annotations,
};
pub(crate) use self::types::is_object_like_type_expression;
pub(crate) use self::union::{
    binary_like_is_type_intersection, binary_like_is_type_union, flatten_binary_like_operands,
    format_type_intersection_binary_layout, format_type_union_binary_layout,
    operator_expression_owns_prefix_annotations, transparent_type_binary_root_expression,
    type_binary_operand_needs_grouping_parentheses, type_union_has_explicit_leading_separator,
    type_union_prefers_inline_assignment_seam, union_has_trailing_own_line_doc_prefix_annotation,
};
