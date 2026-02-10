use std::borrow::Cow;

use destack_ast::{
    Annotation, AnnotationPosition, Argument, AssignOperator, Asynchrony, BinaryOperator, Block,
    Declaration, DeclarationDescriptor, DeclarationKind, Declarator, DependencyItem,
    DependencyKind, DependencyMode, Expression, ForEachBinding, ForEachDeclarationKind,
    ForEachKind, FunctionKind, IfCondition, IfKind, ImportAliasTarget, ImportSource, Keyword,
    LetKind, LocalNodeId, MatchKind, Member, Mutability, NodeTree, NodeType, OperatorPrecedence,
    Parameter, Pattern, PatternField, PostfixPosition, Property, ScalarLiteral, TokenType,
    TypeBinaryOperator, TypeLiteral, TypeModifier, TypePredicateSubject, TypeUnaryOperator,
    UnaryOperator, WhereClause, WhileKind, YieldCardinality,
};
use destack_base::StringId;
use destack_fir::best_fitting;
use destack_fir::format::{FormatError, GroupId, text};
use destack_fir::prelude::*;
use destack_source::Span;
use destack_workspace::TrailingComma;
use smallvec::{SmallVec, smallvec};

use self::binary::*;
use self::call::*;
use self::chain::*;
use self::classify::{
    array_elements_are_fill_candidates, array_has_only_boundary_comments, span_has_comment,
};
use self::control::*;
use self::core::*;
use self::generic::*;
use self::jsx::*;
use self::member::*;
use self::object::*;
use self::operator::*;
use self::parentheses::*;
use self::primary::*;
use self::scan::*;
use self::sort::*;
use self::statement::*;
use self::ternary::*;
use crate::annotation::call_argument_inline_boundary_prefix_annotations;
use crate::argument::list_like;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
    ignore_range_for_node, ignored_node_source,
};
use crate::literal::{format_scalar_literal, format_template_literal};
use crate::property::format_block_of_properties;
use crate::scan::{
    next_non_whitespace_after_annotation, previous_non_whitespace_before_annotation,
};
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};

mod binary;
mod call;
mod chain;
mod classify;
mod control;
mod core;
mod generic;
mod jsx;
mod member;
mod object;
mod operator;
mod parentheses;
mod primary;
mod scan;
mod sort;
mod statement;
mod ternary;

pub(crate) use self::chain::{
    format_expression_chain, is_chain_root, is_expression_chain, is_poorly_breakable_chain,
    lambda_expression_should_break,
};
pub use self::classify::{
    is_complex_argument, is_complex_expression, is_expression_breakable, is_pattern_breakable,
    is_trivial_argument, is_trivial_expression, is_trivial_property,
};
pub(crate) use self::core::format_expression;

// chain head promotion limits
const MAX_CHAIN_HEAD_OPS: usize = 4;
const MAX_CHAIN_HEAD_LEN_DIVISOR: usize = 2;
const DECLARATOR_PREFIX_PADDING: usize = 6;
const ASSIGNMENT_CHAIN_TAIL_RESERVE: usize = 0;

#[cfg(test)]
mod tests;
