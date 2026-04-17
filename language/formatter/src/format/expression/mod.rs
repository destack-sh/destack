mod conditional;
mod control;
mod declarator;
mod dispatch;
mod member;
mod object;
mod parentheses;
mod path;
mod primary;
mod shape;
mod statement;
mod ternary;
mod r#type;

pub(crate) use self::declarator::{
    declarator_drops_parenthesized_value_wrapper, format_declarator,
};
pub(crate) use self::dispatch::{
    format_expression, write_expression_with_prefix_annotations_after_offset,
    write_expression_without_prefix_annotations, write_expression_without_trailing_annotations,
};
pub(crate) use self::member::{
    format_index_expression, format_member_expression, format_type_template_literal,
    postfix_continuation_requires_parenthesized_object_wrapper,
    should_unwrap_parenthesized_member_object,
};
pub(crate) use self::primary::{
    format_primary_expression, write_primary_expression_trailing_annotations,
};
pub(crate) use self::shape::{
    array_elements_are_fill_candidates, array_has_only_boundary_comments,
    expression_has_leading_prefix_comment, expression_has_only_prefix_comment_or_doc_annotations,
    expression_has_prefix_comment_or_doc_annotation_in_left_spine,
    expression_is_trivial_inline_without_annotations, sequence_expression_needs_parens,
};
pub use self::shape::{
    is_expression_breakable, is_pattern_breakable, is_trivial_argument, is_trivial_expression,
    is_trivial_property,
};
pub(crate) use self::statement::{
    format_statement_expression, write_statement_expression_trailing_annotations,
};
pub(crate) use self::ternary::{argument_value, format_expanded_ternary_expression};
pub(crate) use self::r#type::format_block_of_type_members;
pub(crate) use super::operator::{
    expression_has_generic_arguments, format_generic_argument_list,
    format_generic_argument_list_with_relational_spacing,
};
pub(crate) use super::tree::argument_drops_parenthesized_value_wrapper;
