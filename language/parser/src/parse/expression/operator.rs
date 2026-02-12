use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};

use super::common::{
    NOT_IN_FOR_EACH_BINARY_OPERATORS, NOT_IN_STATIC_BINARY_OPERATORS, NOT_IN_TREE_BINARY_OPERATORS,
};

use destack_ast::{
    AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, LocalNodeId, TokenSpan,
    TokenType, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

impl Parser {
    /// Return the current token span or EOF span when unavailable.
    #[inline]
    fn current_span_or_eof(&mut self) -> destack_source::Span {
        self.peek()
            .map(|token| token.span)
            .unwrap_or(self.eof_span())
    }

    /// Return the next token span or EOF span when unavailable.
    #[inline]
    fn next_span_or_eof(&mut self) -> destack_source::Span {
        self.peek_next()
            .map(|token| token.span)
            .unwrap_or(self.eof_span())
    }

    /// Recover a shift operator from adjacent `>` tokens split by type-close scanning.
    fn recover_split_shift_operator(
        &self,
        token: &TokenSpan,
        next_token: Option<&TokenSpan>,
        next_next_token: Option<&TokenSpan>,
    ) -> Option<(InfixOperator, u8)> {
        // this recovery only applies in value expression contexts
        if self.options.in_static || self.options.in_tree_literal || self.options.in_type {
            return None;
        }

        // only `>` can start a recovered right-shift token
        if token.token.ty != TokenType::GreaterThan {
            return None;
        }

        // split recovery only applies when the next tokens are raw `>` and physically adjacent
        let has_adjacent_shift_tokens = next_token
            .filter(|next| next.token.ty == TokenType::GreaterThan)
            .is_some_and(|next| token.span.end == next.span.start);
        if !has_adjacent_shift_tokens {
            return None;
        }

        // `>>>` requires all three `>` tokens to be adjacent
        let has_adjacent_unsigned_shift_tokens =
            next_token
                .zip(next_next_token)
                .is_some_and(|(next, next_next)| {
                    next_next.token.ty == TokenType::GreaterThan
                        && next.span.end == next_next.span.start
                });
        if has_adjacent_unsigned_shift_tokens {
            Some((InfixOperator::Binary(BinaryOperator::UnsignedShiftRight), 3))
        } else {
            Some((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
        }
    }

    /// Make an infix operator from the current parser context.
    fn to_infix_operator(
        &self,
        token_str: &str,
        token: &TokenSpan,
        next_token: Option<&TokenSpan>,
        next_next_token: Option<&TokenSpan>,
        has_newline: bool,
    ) -> ParseResult<(InfixOperator, u8)> {
        // recover split shift operators after type-angle token splitting
        if let Some(operator) =
            self.recover_split_shift_operator(token, next_token, next_next_token)
        {
            return Ok(operator);
        }

        // regular binary operator
        // (only a subset of binary operators are allowed in static and tree contexts)
        if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
            && (!self.options.in_type
                || self.options.in_static
                || matches!(
                    binary_operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ))
            && (!self.options.in_static
                || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.options.in_tree_literal
                || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.options.in_for_each
                || !NOT_IN_FOR_EACH_BINARY_OPERATORS.contains(&binary_operator))
        {
            return Ok((InfixOperator::Binary(binary_operator), 1));
        }

        // regular type binary operator
        // (forbidden in super type clauses, avoid newline glue in TS mode)
        if !self.options.in_super_type
            && let Some(type_binary_operator) =
                TypeBinaryOperator::from_token(token_str, token.token.ty)
            && (!self.options.in_type_mapped_constraint
                || type_binary_operator != TypeBinaryOperator::Cast)
            && (!self.options.in_for_each || type_binary_operator != TypeBinaryOperator::In)
            && (self.options.in_type
                || matches!(
                    type_binary_operator,
                    TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
                )
                || (self.language.is_destack()
                    && matches!(
                        type_binary_operator,
                        TypeBinaryOperator::Extends
                            | TypeBinaryOperator::Implements
                            | TypeBinaryOperator::Is
                    )))
            && (self.language.is_destack() || !has_newline)
        {
            return Ok((InfixOperator::TypeBinary(type_binary_operator), 1));
        }

        // regular assign operator
        // (not allowed in static, type, and tree contexts)
        if !self.options.in_static
            && !self.options.in_type
            && !self.options.in_tree_literal
            && let Some(assign_operator) = AssignOperator::from_token(token.token.ty)
        {
            return Ok((InfixOperator::Assign(assign_operator), 1));
        }

        Err(ParseError::unexpected(token.span))
    }

    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator_maybe(&mut self) -> Option<UnaryOperator> {
        let token = *self.peek().ok()?;
        if token.token.ty == TokenType::Identifier && !self.options.in_type {
            let token_str = self.get_span_str(token.span);
            if let Ok(keyword) = Keyword::from_str(token_str)
                && let Some(operator) = UnaryOperator::from_prefix_keyword(keyword)
            {
                return Some(operator);
            }
        }
        let operator = UnaryOperator::from_prefix_token(token.token.ty)?;

        // dereference (*x) is not valid in JS/TS compatibility mode
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return None;
        }

        // spread (...x) is not a valid standalone expression in JS/TS
        if operator == UnaryOperator::Spread && !self.language.is_destack() {
            return None;
        }

        Some(operator)
    }

    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator(&mut self) -> ParseResult<UnaryOperator> {
        self.peek_unary_prefix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&mut self) -> ParseResult<UnaryOperator> {
        let token = *self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary prefix operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
    }

    /// Peek a type unary operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        self.peek_type_unary_prefix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        let next_token = *self.peek_next().ok()?;
        let token_str = self.get_span_str(token.span);
        let next_token_str = self.get_span_str(next_token.span);
        TypeUnaryOperator::from_postfix_token(token_str, next_token_str, token.token.ty)
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        self.peek_type_unary_postfix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a next type unary operator.
    #[inline]
    pub fn peek_next_type_unary_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        let token = *self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek_next()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Return true when the next token is an assignment operator.
    #[inline]
    pub fn peek_next_assign_operator_is(&mut self) -> bool {
        AssignOperator::from_token(self.peek_next_token_type()).is_some()
    }

    /// Return true when a token index could be an infix or assign operator.
    #[inline]
    pub(super) fn has_infix_or_assign_operator_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);
        if AssignOperator::from_token(token_type).is_some() {
            return true;
        }
        if token_type != TokenType::Identifier {
            return BinaryOperator::from_token("", token_type).is_some();
        }
        if self.has_active_split() {
            return false;
        }
        matches!(
            self.keyword_for_index(index),
            Some(
                Keyword::In
                    | Keyword::InstanceOf
                    | Keyword::As
                    | Keyword::Is
                    | Keyword::Satisfies
                    | Keyword::Extends
                    | Keyword::Implements
            )
        )
    }

    /// Peek an infix operator at a semantic token index.
    #[inline]
    pub(super) fn peek_infix_operator_at_index_maybe(
        &mut self,
        index: usize,
        has_newline: bool,
    ) -> Option<(InfixOperator, u8)> {
        let token = *self.token_ref_at(index)?;
        let (next_token, next_next_token) = if token.token.ty == TokenType::GreaterThan {
            (self.token_at(index + 1), self.token_at(index + 2))
        } else {
            (None, None)
        };
        let token_str = self.get_span_str(token.span);
        self.to_infix_operator(
            token_str,
            &token,
            next_token.as_ref(),
            next_next_token.as_ref(),
            has_newline,
        )
        .ok()
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator_maybe(&mut self) -> Option<(InfixOperator, u8)> {
        self.peek_infix_operator_at_index_maybe(self.pos_index(), false)
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&mut self) -> ParseResult<(InfixOperator, u8)> {
        self.peek_infix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator_maybe(&mut self) -> Option<(InfixOperator, u8)> {
        self.peek_infix_operator_at_index_maybe(self.index_for_next(), true)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&mut self) -> ParseResult<(InfixOperator, u8)> {
        self.peek_next_infix_operator_maybe()
            .ok_or(ParseError::unexpected(self.next_span_or_eof()))
    }

    /// Peek an infix operator after any newlines.
    #[inline]
    pub fn peek_infix_operator_after_newlines_maybe(&mut self) -> Option<(InfixOperator, u8)> {
        let cursor = self.non_newline_cursor_from(self.pos_index().saturating_add(1));
        self.peek_infix_operator_at_index_maybe(cursor.index, true)
    }

    /// Peek an infix operator after any newlines.
    #[inline]
    pub fn peek_infix_operator_after_newlines(&mut self) -> ParseResult<(InfixOperator, u8)> {
        self.peek_infix_operator_after_newlines_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }
    /// Make an expression from an infix operator.
    #[inline]
    pub(super) fn make_infix_expression(
        &self,
        left: LocalNodeId<Expression>,
        operator: InfixOperator,
        right: LocalNodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            InfixOperator::TypeBinary(type_binary_operator) => {
                if self.options.in_type
                    && type_binary_operator == TypeBinaryOperator::Is
                    && let Some(subject) = self.type_predicate_subject_from_expression(left)
                {
                    return Expression::TypePredicate {
                        asserts: false,
                        subject,
                        target: Some(right),
                    };
                }
                Expression::TypeBinary {
                    left,
                    operator: type_binary_operator,
                    right,
                }
            }
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Extract the subject and constraint for a type conditional.
    /// Pull union and intersection chains into the right side when they wrap `extends`.
    /// #Architecture: split_type_conditional_operands is localized reassociation for conditional types.
    /// Since we parse type expressions and expressions in the same pass, we have to post patch type precedence.
    pub(super) fn split_type_conditional_operands(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<(LocalNodeId<Expression>, LocalNodeId<Expression>)> {
        let (operator, left_id, _right_id) = match self.tree.get(expression_id) {
            Expression::TypeBinary {
                operator: TypeBinaryOperator::Extends,
                left,
                right,
            } => {
                return Some((*left, *right));
            }
            Expression::Binary {
                operator,
                left,
                right,
            } => (*operator, *left, *right),
            _ => return None,
        };

        // only normalize union and intersection chains
        if !matches!(
            operator,
            BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
        ) {
            return None;
        }

        // peel a left leaning extends and reattach the union or intersection on the right
        let (extends_left, extends_right) = self.split_type_conditional_operands(left_id)?;
        if let Expression::Binary { left, .. } = self.tree.get_mut(expression_id) {
            *left = extends_right;
        }

        Some((extends_left, expression_id))
    }
}
