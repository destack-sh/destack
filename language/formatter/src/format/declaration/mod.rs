mod assignment;
mod declaration;
mod function;
mod semicolon;
pub(crate) mod sequence;
mod r#type;

pub mod dependency;
pub mod signature;
pub mod statement;

pub(crate) use self::declaration::{
    format_let_statement_expression, format_using_statement_expression,
};
pub(crate) use self::semicolon::{
    expression_needs_statement_terminator, statement_wrapper_needs_semicolon,
};
pub(crate) use self::sequence::expression_is_in_statement_position;
pub(crate) use self::signature::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
pub(crate) use self::r#type::{
    expression_is_decorated_class_declaration, parenthesized_wraps_decorated_class_extends_head,
    parenthesized_wraps_prefix_annotated_class_extends_head,
};
