mod assign;
mod binary;
mod expression;
mod postfix;
mod tokens;
mod r#type;
mod types;
mod union;

pub(crate) use self::assign::{
    AssignmentLikeLayout, assignment_drops_parenthesized_operand_wrapper,
    assignment_rhs_prefers_break_after_operator, write_assignment_like_right,
};
pub(crate) use self::binary::{
    binary_keeps_unary_left_parenthesized_wrapper, format_binary_expression,
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
    format_static_argument_list_with_relational_spacing, should_drop_parenthesized_type_expression,
    write_colon_prefixed_type_annotation, write_type_expression_with_inline_prefix_annotations,
    write_type_expression_with_inline_prefix_annotations_from,
    write_type_expression_without_prefix_annotations,
};
pub(crate) use self::types::is_object_like_type_expression;
pub(crate) use self::union::{
    binary_like_is_type_intersection, binary_like_is_type_union, flatten_type_binary_expression,
    format_type_intersection_binary_layout, format_type_union_binary_layout,
    operator_expression_owns_prefix_annotations, transparent_type_binary_root_expression,
    type_binary_operand_needs_grouping_parentheses, type_union_operand_separator_span,
};
