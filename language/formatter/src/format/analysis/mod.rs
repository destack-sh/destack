pub mod predicate;

pub(crate) use predicate::{
    argument_has_line_comment_annotation, argument_is_collection_literal,
    argument_is_compact_inline_callback, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, argument_is_trivial_unannotated_non_lambda_value,
    call_arguments_are_multiline_span, call_has_static_arguments, first_non_trivia_token_in_span,
    is_call_like_argument, is_simple_static_argument, is_tree_attribute_expression,
    last_non_trivia_token_in_span, next_non_whitespace_token_after_annotation,
    nth_non_trivia_token_in_span, previous_non_whitespace_token_before_annotation,
    previous_non_whitespace_token_before_span, token_is_keyword,
};
