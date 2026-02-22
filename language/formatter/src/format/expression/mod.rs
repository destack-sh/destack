pub(crate) use destack_ast::{
    AnnotationPosition, Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
    DeclarationDescriptor, DeclarationKind, Declarator, DependencyItem, DependencyKind,
    DependencyMode, Expression, ForEachBinding, ForEachDeclarationKind, ForEachKind, FunctionKind,
    IfCondition, IfKind, ImportAliasTarget, ImportSource, Keyword, LetKind, LocalNodeId, MatchKind,
    Member, Mutability, NodeTree, NodeType, OperatorPrecedence, Parameter, Pattern, PatternField,
    PostfixPosition, Property, ScalarLiteral, TokenType, TypeBinaryOperator, TypeLiteral,
    TypeModifier, TypePredicateSubject, TypeUnaryOperator, UnaryOperator, WhereClause, WhileKind,
    YieldCardinality,
};
pub(crate) use destack_base::StringId;
pub(crate) use destack_fir::format::{FormatError, GroupId};
pub(crate) use destack_fir::prelude::*;
pub(crate) use destack_source::Span;
pub(crate) use destack_workspace::TrailingComma;
pub(crate) use smallvec::SmallVec;

pub(crate) use self::control::*;
use self::declarator::format_declarator;
pub(crate) use self::format::{
    array_elements_are_fill_candidates, array_has_only_boundary_comments, span_has_comment,
};
pub(crate) use self::member::*;
pub(crate) use self::object::*;
pub(crate) use self::parentheses::*;
use self::primary::format_primary_expression;
use self::statement::format_statement_expression;
pub(crate) use self::ternary::*;
pub(crate) use crate::format::analysis::{
    ArgumentSimplicityOptions, argument_has_non_blank_annotation,
    argument_is_inline_closure_cast_object, argument_is_simple_with_options,
    call_arguments_are_multiline_span, is_call_like_argument, is_simple_static_argument,
    is_tree_attribute_expression,
};
pub(crate) use crate::format::call::{format_call_expression, format_instantiation_expression};
pub(crate) use crate::format::chain::{
    argument_value_id, chain_nodes, expression_is_in_template_literal_interpolation,
    has_comment_between_expressions, has_line_comment_between_expressions,
    is_block_lambda_argument, is_chain_root, is_expression_chain, is_lambda_expression,
    is_poorly_breakable_chain, lambda_expression_should_break, member_has_intervening_comment,
    should_force_multiline_mapped_type, should_parenthesize_index_expression,
    transparent_inner_expression,
};
pub(crate) use crate::format::collection::list_like;
pub(crate) use crate::format::collection::literal::{
    format_scalar_literal, format_template_literal,
};
pub(crate) use crate::format::collection::property::format_block_of_properties;
pub(crate) use crate::format::operator::{
    flattened_binary_operand_count, is_chain_expression, needs_parens_in_postfix_position,
    write_postfix_base_expression,
};
pub(crate) use crate::format::tree::{
    HugOptions, argument_is_array_literal, argument_is_block_callback,
    argument_is_function_expression, argument_is_lambda_expression, argument_is_object_literal,
    argument_is_template_literal, format_hugged, property_has_complex_type_value,
    property_has_complex_value, tree_literal_should_break, tree_literal_should_expand,
};
pub(crate) use crate::{Annotation, DestackFormatContext, DestackFormatter};

#[cfg(test)]
pub(crate) use crate::format::tree::expression_has_complex_callback;

mod control;
mod declarator;
mod format;
mod member;
mod object;
mod parentheses;
mod primary;
mod statement;
mod ternary;

pub(crate) use self::format::*;
pub use self::format::{
    is_complex_argument, is_complex_expression, is_expression_breakable, is_pattern_breakable,
    is_trivial_argument, is_trivial_expression, is_trivial_property,
};

#[cfg(test)]
mod tests;
