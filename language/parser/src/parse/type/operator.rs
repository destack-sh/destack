use destack_dir::{Keyword, OperatorPrecedence, RangeEnd, TokenType};

/// One type prefix operation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TypePrefixOperator {
    /// `keyof T`.
    Keyof,
    /// `readonly T`.
    Readonly,
    /// `!T`.
    Not,
}

impl TypePrefixOperator {
    /// Classify one type prefix token and optional identifier keyword.
    #[inline]
    pub(in crate::parse) fn from_token(token: TokenType, keyword: Option<Keyword>) -> Option<Self> {
        match token {
            TokenType::Not => Some(Self::Not),
            TokenType::Identifier => match keyword? {
                Keyword::Keyof => Some(Self::Keyof),
                Keyword::Readonly => Some(Self::Readonly),
                _ => None,
            },
            _ => None,
        }
    }
}

/// One type relation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TypeRelation {
    /// `extends`.
    Extends,
    /// `implements`.
    Implements,
}

/// One type infix operation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(in crate::parse) enum TypeOperator {
    /// A union operation.
    Union,
    /// An intersection operation.
    Intersection,
    /// A type relation operation.
    Relation(TypeRelation),
    /// A type range operation.
    Range(RangeEnd),
}

impl TypeOperator {
    /// Classify one type infix token and optional identifier keyword.
    #[inline]
    pub(in crate::parse) fn from_token(token: TokenType, keyword: Option<Keyword>) -> Option<Self> {
        let operator = match token {
            TokenType::ElementwiseOr => Self::Union,
            TokenType::ElementwiseAnd => Self::Intersection,
            TokenType::Range => Self::Range(RangeEnd::Open),
            TokenType::RangeInclusive => Self::Range(RangeEnd::Inclusive),
            TokenType::Identifier => match keyword? {
                Keyword::Extends => Self::Relation(TypeRelation::Extends),
                Keyword::Implements => Self::Relation(TypeRelation::Implements),
                _ => return None,
            },
            _ => return None,
        };

        Some(operator)
    }

    /// Return the precedence of this operation.
    pub(in crate::parse) const fn precedence(self) -> OperatorPrecedence {
        match self {
            TypeOperator::Union => OperatorPrecedence::BitwiseOr,
            TypeOperator::Intersection => OperatorPrecedence::BitwiseAnd,
            TypeOperator::Relation(_) => OperatorPrecedence::TypeRelation,
            TypeOperator::Range(_) => OperatorPrecedence::Range,
        }
    }
}
