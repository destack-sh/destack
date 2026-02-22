pub(crate) use crate::format::analysis::call_has_non_blank_infix_annotation;
pub(crate) use crate::format::call::{
    call_arguments_force_expand_for_chain, format_call_arguments,
};
pub(crate) use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, ArgumentSimplicityOptions, BinaryOperands,
    Declaration, DestackFormatContext, Expression, FormatError, FormatResult, FunctionKind, IfKind,
    LocalNodeId, NodeTree, NodeType, ParenthesizedUnwrapPolicy, PostfixPosition, ScalarLiteral,
    SmallVec, Span, TokenType, argument_has_non_blank_annotation, argument_is_function_expression,
    argument_is_inline_closure_cast_object, argument_is_simple_with_options,
    argument_is_template_literal, flatten_binary_expression, flatten_type_binary_expression,
    flattened_binary_operand_count, format_call_expression, format_index_expression,
    format_instantiation_expression, format_member_expression, is_call_like_argument,
    is_chain_expression, is_expression_breakable, is_trivial_expression,
    needs_parens_in_postfix_position, parenthesized_should_unwrap, span_has_comment,
    tree_literal_should_break,
};

mod base;
mod r#break;
mod classify;
mod format;
mod line_group;
mod normalize;

pub(crate) use self::base::*;
pub(crate) use self::r#break::*;
pub(crate) use self::classify::*;
pub(crate) use self::format::*;
