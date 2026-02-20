mod assign;
mod binary;
mod common;
mod context;
mod dispatch;
mod r#new;
mod token;

pub(crate) use self::context::{
    expression_static_arguments, format_binary_operand_with_grouping_parentheses,
    format_binary_operand_without_prefix_annotations_with_grouping_parentheses,
    is_object_like_type_expression, is_parameter_type_annotation,
    is_simple_type_binary_left_expression, is_static_type_argument_context, is_type_context,
    should_hug_nullable_union_type, should_hug_static_argument_union_type,
    type_binary_operand_needs_grouping_parentheses, union_source_has_leading_pipe,
};
pub(crate) use self::dispatch::format_operator_expression;
pub(crate) use crate::expression::{
    Annotation, AnnotationPosition, AssignOperator, BinaryOperands, BinaryOperator,
    DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId, NodeType,
    ParenthesizedDropPolicy, Span, TokenType, expression_has_leading_prefix_comment,
    expression_has_non_doc_multiline_block_prefix_comment_annotation,
    has_comment_between_expressions, is_trivial_expression, parenthesized_should_drop, space,
    span_has_comment,
};
