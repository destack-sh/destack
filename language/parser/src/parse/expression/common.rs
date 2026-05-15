use crate::Parser;

use destack_dir::{
    BinaryOperator, Declaration, ExportKind, Expression, FunctionDeclaration, FunctionForm,
    GenericArgument, Keyword, LocalNodeId, TokenType,
};
use destack_source::Span;

use super::super::PendingDecorators;

pub static DECLARATION_START_TOKENS: [TokenType; 6] = [
    TokenType::Literal,
    TokenType::Identifier,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::LessThan,
];

pub static PATTERN_START_TOKENS: [TokenType; 6] = [
    TokenType::Identifier,
    TokenType::Literal,
    TokenType::ElementwiseAnd,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
];

// can't use anything with `<` or `>` in generic arguments
// (to avoid parsing ambiguity with `<>` brackets)
pub(super) static NOT_IN_GENERIC_ARGUMENT_BINARY_OPERATORS: [BinaryOperator; 7] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::ShiftRight,
    BinaryOperator::UnsignedShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
];

// can't use anything with `<` or `>` in tree fragments
pub(super) static NOT_IN_TREE_BINARY_OPERATORS: [BinaryOperator; 8] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::ShiftRight,
    BinaryOperator::UnsignedShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
    // multiply
    BinaryOperator::Divide,
];

// can't use `in` in for each expressions
pub(super) static NOT_IN_FOR_EACH_BINARY_OPERATORS: [BinaryOperator; 1] = [BinaryOperator::In];

/// Parsed declaration prefix shared across declaration forms.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub(crate) struct DeclarationHeader {
    /// The export kind for the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The explicit `declare` modifier span.
    pub declare_span: Option<Span>,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// Whether the declaration is final.
    pub is_final: bool,
    /// Whether the declaration has shared placement.
    pub is_shared: bool,
}

/// Result of parsing declaration modifiers.
pub(super) enum DescriptorHead {
    /// Parsed declaration header.
    Header {
        /// The parsed declaration header.
        header: DeclarationHeader,
        /// Decorators parsed between declaration modifiers and the declaration head.
        decorators: PendingDecorators,
    },
    /// Parsed expression that consumed the modifiers.
    Expression(LocalNodeId<Expression>),
}

/// Return whether a keyword is one of the type-relation operators in expression position.
#[inline]
pub(super) fn is_type_relation_keyword(keyword: Option<Keyword>) -> bool {
    matches!(
        keyword,
        Some(
            Keyword::As
                | Keyword::Satisfies
                | Keyword::Extends
                | Keyword::Implements
                | Keyword::In
                | Keyword::InstanceOf
                | Keyword::Is
        )
    )
}

/// Return true when a keyword starts a declaration.
#[inline]
pub(super) fn is_declaration_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::Declare
            | Keyword::Abstract
            | Keyword::Struct
            | Keyword::Class
            | Keyword::Enum
            | Keyword::Union
            | Keyword::Function
            | Keyword::Extension
            | Keyword::Interface
            | Keyword::Type
            | Keyword::Newtype
            | Keyword::Const
            | Keyword::Readonly
            | Keyword::Let
            | Keyword::Using
            | Keyword::Override
            | Keyword::Public
            | Keyword::Protected
            | Keyword::Private
            | Keyword::Async
    )
}

impl Parser {
    /// Unwrap label wrappers to get the underlying expression.
    pub(crate) fn unwrap_label_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;
        loop {
            let expression = self.tree.get(current);
            match expression {
                Expression::Label { body, .. } => {
                    current = *body;
                }
                _ => break,
            }
        }
        current
    }

    /// Split one instantiation expression into its receiver and generic arguments.
    pub(crate) fn split_instantiation_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> (LocalNodeId<Expression>, Vec<LocalNodeId<GenericArgument>>) {
        match self.tree.get(expression_id) {
            Expression::Instantiation {
                left,
                generic_arguments,
            } => (*left, generic_arguments.clone()),
            _ => (expression_id, Vec::new()),
        }
    }

    /// Return true when an expression is a lambda declaration without wrapping parentheses.
    pub(crate) fn is_unparenthesized_lambda_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            self.tree.get(expression_id),
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.form == FunctionForm::Lambda
                )
        )
    }

    /// Return true when an expression can be used as an unparenthesized tagged template tag.
    pub(super) fn tagged_template_tag_is_valid(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // unparenthesized lambdas cannot be tagged template receivers
        if self.is_unparenthesized_lambda_expression(expression_id) {
            return false;
        }

        // unparenthesized unary expressions are not valid tagged template receivers
        !matches!(self.tree.get(expression_id), Expression::Unary { .. })
    }
}
