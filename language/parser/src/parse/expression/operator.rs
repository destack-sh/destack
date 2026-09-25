use crate::parse::r#type::operator::TypeRelation;
use tspp_dir::{AssignOperator, BinaryOperator, Keyword, OperatorPrecedence, RangeEnd, TokenType};

/// One value infix operation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(in crate::parse) enum ExpressionOperator {
    /// A value binary operation.
    Binary(BinaryOperator),
    /// A type relation entered from value syntax.
    Type(TypeRelation),
    /// A value assignment operation.
    Assign(AssignOperator),
    /// A runtime type predicate.
    Is,
    /// A runtime constructor predicate.
    InstanceOf,
    /// A type assertion.
    As,
    /// A type conformance assertion.
    Satisfies,
    /// A value range operation.
    Range(RangeEnd),
}

impl ExpressionOperator {
    /// Classify one value infix token and optional identifier keyword.
    #[inline]
    pub(in crate::parse) fn from_token(token: TokenType, keyword: Option<Keyword>) -> Option<Self> {
        if let Some(operator) = AssignOperator::from_token(token) {
            return Some(Self::Assign(operator));
        }

        let operator = match token {
            TokenType::Multiply => Self::Binary(BinaryOperator::Multiply),
            TokenType::Exponent => Self::Binary(BinaryOperator::Exponent),
            TokenType::Divide => Self::Binary(BinaryOperator::Divide),
            TokenType::Remainder => Self::Binary(BinaryOperator::Remainder),
            TokenType::Add => Self::Binary(BinaryOperator::Add),
            TokenType::Subtract => Self::Binary(BinaryOperator::Subtract),
            TokenType::ShiftLeft => Self::Binary(BinaryOperator::ShiftLeft),
            TokenType::ShiftRight => Self::Binary(BinaryOperator::ShiftRight),
            TokenType::UnsignedShiftRight => Self::Binary(BinaryOperator::UnsignedShiftRight),
            TokenType::ElementwiseAnd => Self::Binary(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Self::Binary(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Self::Binary(BinaryOperator::ElementwiseOr),
            TokenType::Equal => Self::Binary(BinaryOperator::Equal),
            TokenType::EqualWide => Self::Binary(BinaryOperator::EqualStrict),
            TokenType::NotEqual => Self::Binary(BinaryOperator::NotEqual),
            TokenType::NotEqualWide => Self::Binary(BinaryOperator::NotEqualStrict),
            TokenType::LessThan => Self::Binary(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Self::Binary(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Self::Binary(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Self::Binary(BinaryOperator::GreaterThanOrEqual),
            TokenType::LogicalAnd => Self::Binary(BinaryOperator::And),
            TokenType::LogicalOr => Self::Binary(BinaryOperator::Or),
            TokenType::Coalesce => Self::Binary(BinaryOperator::Coalesce),
            TokenType::Range => Self::Range(RangeEnd::Open),
            TokenType::RangeInclusive => Self::Range(RangeEnd::Inclusive),
            TokenType::Identifier => match keyword? {
                Keyword::In => Self::Binary(BinaryOperator::In),
                Keyword::InstanceOf => Self::InstanceOf,
                Keyword::As => Self::As,
                Keyword::Is => Self::Is,
                Keyword::Satisfies => Self::Satisfies,
                Keyword::Extends => Self::Type(TypeRelation::Extends),
                Keyword::Implements => Self::Type(TypeRelation::Implements),
                _ => return None,
            },
            _ => return None,
        };

        Some(operator)
    }

    /// Return the precedence of this operation.
    pub(in crate::parse) const fn precedence(self) -> OperatorPrecedence {
        match self {
            ExpressionOperator::Binary(operator) => operator.precedence(),
            ExpressionOperator::Type(_) => OperatorPrecedence::TypeRelation,
            ExpressionOperator::Assign(operator) => operator.precedence(),
            ExpressionOperator::Is
            | ExpressionOperator::InstanceOf
            | ExpressionOperator::As
            | ExpressionOperator::Satisfies => OperatorPrecedence::Comparison,
            ExpressionOperator::Range(_) => OperatorPrecedence::Range,
        }
    }

    /// Return whether this operation consumes a type operand.
    pub(in crate::parse) const fn has_type_operand(self) -> bool {
        matches!(
            self,
            ExpressionOperator::Is | ExpressionOperator::As | ExpressionOperator::Satisfies
        )
    }
}
