pub(crate) use std::borrow::Cow;

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
pub(crate) use destack_fir::format::{FormatError, GroupId, text};
pub(crate) use destack_fir::prelude::*;
pub(crate) use destack_source::Span;
pub(crate) use destack_workspace::TrailingComma;
pub(crate) use smallvec::SmallVec;

pub(crate) use self::classify::{
    array_elements_are_fill_candidates, array_has_only_boundary_comments, span_has_comment,
};
pub(crate) use self::control::*;
pub(self) use self::declarator::*;
pub(crate) use self::generic::*;
pub(crate) use self::member::*;
pub(crate) use self::object::*;
pub(crate) use self::parentheses::*;
pub(self) use self::primary::*;
pub(crate) use self::scan::*;
pub(self) use self::sort::*;
pub(self) use self::statement::*;
pub(crate) use self::ternary::*;
pub(crate) use crate::analysis::*;
pub(crate) use crate::call::*;
pub(crate) use crate::chain::*;
pub(crate) use crate::collection::list_like;
pub(crate) use crate::collection::literal::{format_scalar_literal, format_template_literal};
pub(crate) use crate::collection::property::format_block_of_properties;
pub(crate) use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
    ignored_node_source,
};
pub(crate) use crate::tree::*;
pub(crate) use crate::{
    Annotation, DestackFormatContext, DestackFormatter, FormatNode,
    empty_block_with_infix_annotations,
};

mod classify;
mod control;
mod core;
mod declarator;
mod generic;
mod member;
mod object;
mod parentheses;
mod primary;
mod scan;
mod sort;
mod statement;
mod ternary;

pub use self::classify::{
    is_complex_argument, is_complex_expression, is_expression_breakable, is_pattern_breakable,
    is_trivial_argument, is_trivial_expression, is_trivial_property,
};
pub(crate) use self::core::format_expression;
pub(crate) use self::scan::{
    expression_has_non_doc_multiline_block_prefix_comment_annotation,
    expression_has_static_type_arguments, source_min_inline_char_len,
};

#[cfg(test)]
mod tests;
