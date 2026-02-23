pub mod predicate;
pub mod timing;

pub(crate) use predicate::{
    ArgumentSimplicityOptions, argument_has_leading_prefix_annotation_outside_span,
    argument_has_line_comment_annotation, argument_has_multiline_prefix_annotation,
    argument_has_separator_line_comment_annotation, argument_is_collection_literal,
    argument_is_inline_closure_cast_object, argument_is_interpolated_template_literal,
    argument_is_simple_with_options, call_arguments_are_multiline_span,
    call_arguments_have_boundary_comments, call_arguments_preserve_blank_line_between,
    call_has_leading_block_callback_with_simple_tail, call_has_static_arguments,
    first_non_trivia_token_in_span, is_call_like_argument, is_simple_static_argument,
    is_tree_attribute_expression, last_non_trivia_token_in_span,
    next_non_whitespace_after_annotation, next_non_whitespace_after_span,
    next_non_whitespace_token_after_annotation, next_non_whitespace_token_after_span,
    previous_non_whitespace_before_annotation, previous_non_whitespace_token_before_annotation,
    previous_non_whitespace_token_before_span, token_is_keyword,
};
