use crate::Parser;

use destack_ast::{
    BinaryOperator, Declaration, DeclarationDescriptor, Expression, FunctionKind, Keyword,
    LocalNodeId, TokenType,
};

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

// can't use anything with `<` or `>` in static arguments
// (to avoid parsing ambiguity with `<>` brackets)
pub(super) static NOT_IN_STATIC_BINARY_OPERATORS: [BinaryOperator; 8] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
    BinaryOperator::ShiftRight,
    BinaryOperator::UnsignedShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
];

// can't use anything with `<` or `>` in tree fragments
pub(super) static NOT_IN_TREE_BINARY_OPERATORS: [BinaryOperator; 9] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
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

/// Result of parsing declaration modifiers.
pub(super) enum DescriptorHead {
    /// Parsed declaration descriptor.
    Descriptor(DeclarationDescriptor),
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
            | Keyword::Namespace
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
            | Keyword::Var
            | Keyword::Using
            | Keyword::Override
            | Keyword::Public
            | Keyword::Protected
            | Keyword::Private
            | Keyword::Async
    )
}

impl Parser {
    /// Unwrap statement and label wrappers to get the underlying expression.
    pub(crate) fn unwrap_statement_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;
        loop {
            let expression = self.tree.get(current);
            match expression {
                Expression::Statement(inner) => {
                    current = *inner;
                }
                Expression::Labelled { body, .. } => {
                    current = *body;
                }
                _ => break,
            }
        }
        current
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
                    Declaration::Function { signature, .. }
                        if signature.kind == FunctionKind::Lambda
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
