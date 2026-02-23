pub(crate) use crate::format::call::{
    call_arguments_force_expand_for_chain, format_call_arguments,
};
pub(crate) use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, ArgumentSimplicityOptions, Declaration,
    DestackFormatContext, Expression, FormatError, FormatResult, FunctionKind, IfKind, LocalNodeId,
    NodeTree, NodeType, ParenthesizedUnwrapMode, PostfixPosition, ScalarLiteral, SmallVec, Span,
    TokenType, argument_is_function_expression, argument_is_inline_closure_cast_object,
    argument_is_simple_with_options, argument_is_template_literal, format_call_expression,
    format_index_expression, format_instantiation_expression, format_member_expression,
    is_call_like_argument, is_expression_breakable, should_unwrap_parenthesized,
    tree_literal_should_break,
};
pub(crate) use crate::format::operator::{
    BinaryOperands, flatten_binary_expression, flatten_type_binary_expression,
    flattened_binary_operand_count, is_chain_expression, needs_parens_in_postfix_position,
};

mod analysis;
mod base;
mod r#break;
mod format;
mod normalize;

pub(crate) use self::analysis::*;
pub(crate) use self::base::*;
pub(crate) use self::r#break::*;
pub(crate) use self::format::*;
