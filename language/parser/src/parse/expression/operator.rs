use crate::Parser;
use crate::parse::r#type::operator::TypeBinaryOperator;
use destack_dir::{
    AssignOperator, BinaryOperator, Keyword, OperatorPrecedence, TokenType, UnaryOperator,
};

/// One value expression infix operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(in crate::parse) enum ExpressionInfixOperator {
    /// Value binary operator.
    Binary(BinaryOperator),
    /// Type relation operator parsed from value space.
    TypeBinary(TypeBinaryOperator),
    /// Value assignment operator.
    Assign(AssignOperator),
    /// Runtime type predicate.
    Is,
    /// Runtime constructor predicate.
    InstanceOf,
    /// TypeScript `as` assertion.
    As,
    /// TypeScript `satisfies` assertion.
    Satisfies,
    /// Destack range operator.
    Range(destack_dir::RangeEnd),
}

impl ExpressionInfixOperator {
    /// Return this operator precedence.
    pub(in crate::parse) fn precedence(self) -> u16 {
        match self {
            Self::Binary(operator) => operator.precedence(),
            Self::TypeBinary(operator) => operator.precedence(),
            Self::Assign(operator) => operator.precedence(),
            Self::Is | Self::InstanceOf | Self::As | Self::Satisfies => {
                OperatorPrecedence::Comparison as u16
            }
            Self::Range(_) => OperatorPrecedence::Range as u16,
        }
    }

    /// Return whether this operator groups right to left.
    pub(in crate::parse) fn is_right_associative(self) -> bool {
        matches!(self, Self::Assign(_))
    }
}

impl Parser {
    /// Return a direct value binary operator for one token type.
    #[inline]
    fn value_binary_operator_from_token_type(token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::Exponent => Some(BinaryOperator::Exponent),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),
            TokenType::UnsignedShiftRight => Some(BinaryOperator::UnsignedShiftRight),
            TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Some(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::EqualWide => Some(BinaryOperator::EqualStrict),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::NotEqualWide => Some(BinaryOperator::NotEqualStrict),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),
            TokenType::Coalesce => Some(BinaryOperator::Coalesce),
            _ => None,
        }
    }

    /// Return a value prefix operator at the current token.
    pub fn peek_unary_prefix_operator_maybe(&mut self) -> Option<UnaryOperator> {
        let token_type = self.peek_token_type();
        if token_type == TokenType::Identifier {
            return self
                .current_keyword()
                .and_then(UnaryOperator::from_prefix_keyword);
        }

        let operator = UnaryOperator::from_prefix_token(token_type)?;
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return None;
        }

        Some(operator)
    }

    /// Return true when the next token is an assignment operator.
    pub fn peek_next_assign_operator_is(&mut self) -> bool {
        AssignOperator::from_token(self.next_token_type()).is_some()
    }

    /// Return true when the current token could start an infix or assignment operator.
    pub(crate) fn current_token_can_start_infix_or_assign_operator(&mut self) -> bool {
        if AssignOperator::from_token(self.peek_token_type()).is_some() {
            return true;
        }

        self.peek_infix_operator_maybe().is_some()
    }

    /// Return an infix operator at the current token.
    pub(super) fn peek_infix_operator_maybe(&mut self) -> Option<ExpressionInfixOperator> {
        let token_type = self.peek_token_type();

        self.infix_operator_from_current_token_type(token_type)
    }

    /// Return an infix operator at one token offset.
    pub(in crate::parse) fn peek_infix_operator_at_offset_maybe(
        &mut self,
        offset: usize,
    ) -> Option<ExpressionInfixOperator> {
        let token = self.token_at_offset(offset);
        let token_type = token.token.ty;

        Self::infix_operator_from_token(self.language.is_destack(), token_type, || {
            self.keyword_at_offset(offset)
        })
    }

    /// Return the parser infix operator for the current token type.
    pub(super) fn infix_operator_from_current_token_type(
        &self,
        token_type: TokenType,
    ) -> Option<ExpressionInfixOperator> {
        Self::infix_operator_from_token(self.language.is_destack(), token_type, || {
            self.current_keyword()
        })
    }

    /// Return the parser infix operator for one token and optional keyword.
    fn infix_operator_from_token(
        is_destack: bool,
        token_type: TokenType,
        keyword: impl FnOnce() -> Option<Keyword>,
    ) -> Option<ExpressionInfixOperator> {
        if let Some(operator) = AssignOperator::from_token(token_type) {
            return Some(ExpressionInfixOperator::Assign(operator));
        }

        if is_destack && token_type == TokenType::Range {
            return Some(ExpressionInfixOperator::Range(destack_dir::RangeEnd::Open));
        }

        if is_destack && token_type == TokenType::RangeInclusive {
            return Some(ExpressionInfixOperator::Range(
                destack_dir::RangeEnd::Inclusive,
            ));
        }

        if let Some(operator) = Self::value_binary_operator_from_token_type(token_type) {
            return Some(ExpressionInfixOperator::Binary(operator));
        }

        if token_type != TokenType::Identifier {
            return None;
        }

        let operator = match keyword()? {
            Keyword::In => ExpressionInfixOperator::Binary(BinaryOperator::In),
            Keyword::InstanceOf => ExpressionInfixOperator::InstanceOf,
            Keyword::As => ExpressionInfixOperator::As,
            Keyword::Is => ExpressionInfixOperator::Is,
            Keyword::Satisfies => ExpressionInfixOperator::Satisfies,
            Keyword::Extends => ExpressionInfixOperator::TypeBinary(TypeBinaryOperator::Extends),
            Keyword::Implements => {
                ExpressionInfixOperator::TypeBinary(TypeBinaryOperator::Implements)
            }
            _ => return None,
        };

        Some(operator)
    }
}
