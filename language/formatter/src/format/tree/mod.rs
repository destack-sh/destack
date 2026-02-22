mod argument;
mod literal;

pub(crate) use self::argument::{
    HugOptions, argument_is_array_literal, argument_is_block_callback,
    argument_is_function_expression, argument_is_lambda_expression, argument_is_object_literal,
    argument_is_template_literal, format_hugged, has_multiline_jsx_argument,
    property_has_complex_type_value, property_has_complex_value,
    tree_argument_is_wrapped_in_braces,
};
pub(crate) use self::literal::{
    format_tree_literal_expression, tree_literal_should_break, tree_literal_should_expand,
};

#[cfg(test)]
pub(crate) use self::argument::expression_has_complex_callback;
