use crate::Parser;
use crate::parse::r#type::operator::TypeBinaryOperator;
use destack_dir::{
    AssignOperator, BinaryOperator, Keyword, OperatorPrecedence, RangeEnd, TokenType, UnaryOperator,
};

/// One classified value infix operator with binding power.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(in crate::parse::expression) struct CurrentExpressionInfixOperator {
    /// The classified operator.
    pub(in crate::parse::expression) operator: ExpressionInfixOperator,
    /// The Pratt binding power.
    pub(in crate::parse::expression) precedence: u16,
    /// Whether this operator groups right to left.
    pub(in crate::parse::expression) is_right_associative: bool,
}

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
    /// Return a classified binary operator with precedence for one token type.
    #[inline]
    fn value_binary_infix_from_token_type(
        token_type: TokenType,
    ) -> Option<CurrentExpressionInfixOperator> {
        let operator = match token_type {
            TokenType::Multiply => BinaryOperator::Multiply,
            TokenType::Exponent => BinaryOperator::Exponent,
            TokenType::Divide => BinaryOperator::Divide,
            TokenType::Remainder => BinaryOperator::Remainder,
            TokenType::Add => BinaryOperator::Add,
            TokenType::Subtract => BinaryOperator::Subtract,
            TokenType::ShiftLeft => BinaryOperator::ShiftLeft,
            TokenType::ShiftRight => BinaryOperator::ShiftRight,
            TokenType::UnsignedShiftRight => BinaryOperator::UnsignedShiftRight,
            TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
            TokenType::ElementwiseXor => BinaryOperator::ElementwiseXor,
            TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
            TokenType::Equal => BinaryOperator::Equal,
            TokenType::EqualWide => BinaryOperator::EqualStrict,
            TokenType::NotEqual => BinaryOperator::NotEqual,
            TokenType::NotEqualWide => BinaryOperator::NotEqualStrict,
            TokenType::LessThan => BinaryOperator::LessThan,
            TokenType::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
            TokenType::GreaterThan => BinaryOperator::GreaterThan,
            TokenType::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
            TokenType::LogicalAnd => BinaryOperator::And,
            TokenType::LogicalOr => BinaryOperator::Or,
            TokenType::Coalesce => BinaryOperator::Coalesce,
            _ => return None,
        };

        Some(CurrentExpressionInfixOperator {
            operator: ExpressionInfixOperator::Binary(operator),
            precedence: operator.precedence(),
            is_right_associative: false,
        })
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
        self.current_infix_operator_maybe().is_some()
    }

    /// Return an infix operator at the current token.
    pub(super) fn peek_infix_operator_maybe(&mut self) -> Option<ExpressionInfixOperator> {
        self.current_infix_operator_maybe()
            .map(|operator| operator.operator)
    }

    /// Return an infix operator at the current token with binding power.
    #[inline]
    pub(in crate::parse::expression) fn current_infix_operator_maybe(
        &self,
    ) -> Option<CurrentExpressionInfixOperator> {
        Self::infix_operator_from_token(
            self.language.is_destack(),
            self.current_token().ty(),
            self.current_token().keyword(),
        )
    }

    /// Return a parser infix operator for one token and optional keyword.
    #[inline]
    fn infix_operator_from_token(
        is_destack: bool,
        token_type: TokenType,
        keyword: Option<Keyword>,
    ) -> Option<CurrentExpressionInfixOperator> {
        if let Some(operator) = AssignOperator::from_token(token_type) {
            return Some(CurrentExpressionInfixOperator {
                operator: ExpressionInfixOperator::Assign(operator),
                precedence: operator.precedence(),
                is_right_associative: true,
            });
        }

        if is_destack && token_type == TokenType::Range {
            return Some(CurrentExpressionInfixOperator {
                operator: ExpressionInfixOperator::Range(RangeEnd::Open),
                precedence: OperatorPrecedence::Range as u16,
                is_right_associative: false,
            });
        }

        if is_destack && token_type == TokenType::RangeInclusive {
            return Some(CurrentExpressionInfixOperator {
                operator: ExpressionInfixOperator::Range(RangeEnd::Inclusive),
                precedence: OperatorPrecedence::Range as u16,
                is_right_associative: false,
            });
        }

        if let Some(operator) = Self::value_binary_infix_from_token_type(token_type) {
            return Some(operator);
        }

        if token_type != TokenType::Identifier {
            return None;
        }

        match keyword? {
            Keyword::In => Some(CurrentExpressionInfixOperator {
                operator: ExpressionInfixOperator::Binary(BinaryOperator::In),
                precedence: BinaryOperator::In.precedence(),
                is_right_associative: false,
            }),
            Keyword::InstanceOf => Some(Self::keyword_infix(ExpressionInfixOperator::InstanceOf)),
            Keyword::As => Some(Self::keyword_infix(ExpressionInfixOperator::As)),
            Keyword::Is => Some(Self::keyword_infix(ExpressionInfixOperator::Is)),
            Keyword::Satisfies => Some(Self::keyword_infix(ExpressionInfixOperator::Satisfies)),
            Keyword::Extends => Some(Self::keyword_infix(ExpressionInfixOperator::TypeBinary(
                TypeBinaryOperator::Extends,
            ))),
            Keyword::Implements => Some(Self::keyword_infix(ExpressionInfixOperator::TypeBinary(
                TypeBinaryOperator::Implements,
            ))),
            _ => None,
        }
    }

    /// Return one non-assignment keyword operator record.
    #[inline]
    fn keyword_infix(operator: ExpressionInfixOperator) -> CurrentExpressionInfixOperator {
        CurrentExpressionInfixOperator {
            operator,
            precedence: operator.precedence(),
            is_right_associative: false,
        }
    }
}
