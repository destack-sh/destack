mod declaration;
mod function;
mod lambda;
mod semicolon;
pub(crate) mod sequence;
mod r#type;

pub mod dependency;
pub mod signature;
pub mod statement;

pub(crate) use self::declaration::{
    format_let_else_statement_expression, format_let_statement_expression,
    format_using_statement_expression,
};
pub(crate) use self::function::format_function_declaration;
pub(crate) use self::lambda::{
    FormatLambdaDeclarationOptions, FunctionCacheMode, GroupedCallArgumentLayout,
    format_lambda_declaration, format_lambda_declaration_with_options,
};
pub(crate) use self::semicolon::{
    expression_needs_statement_terminator, statement_trailing_comment_anchor_end,
    statement_wrapper_needs_semicolon, write_statement_terminator,
    write_statement_terminator_after_anchor, write_statement_terminator_with_following_start,
};
pub(crate) use self::sequence::expression_is_in_statement_position;
pub(crate) use self::statement::empty_block_with_infix_annotations;
