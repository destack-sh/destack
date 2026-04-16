use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Expression, Keyword, LiteralType, LocalNodeId, NodeType, NumberBase, ScalarLiteral, TokenType,
    TypeExpression,
};
use destack_core::StringId;
use destack_source::Span;

impl Parser {
    /// Return true when an expression is a decimal integer token directly before `.`.
    #[inline]
    pub(super) fn expression_is_decimal_integer_before_dot(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        dot_index: usize,
    ) -> bool {
        // only integer scalar literals can use decimal separators for member access
        if !matches!(
            self.tree.get(left_expression_id),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) {
            return false;
        }

        // the candidate must be a dot token adjacent to the literal span
        let left_span = self.tree.get_span(left_expression_id);
        let Some(dot_token) = self.token_ref_at(dot_index).copied() else {
            return false;
        };
        if dot_token.token.ty != TokenType::Dot || left_span.end != dot_token.span.start {
            return false;
        }

        // the left expression must map to one decimal integer literal token
        let Some(literal_index) = dot_index.checked_sub(1) else {
            return false;
        };
        let Some(literal_token) = self.token_ref_at(literal_index).copied() else {
            return false;
        };
        if literal_token.span != left_span || literal_token.token.ty != TokenType::Literal {
            return false;
        }

        matches!(
            literal_token.token.literal,
            Some(LiteralType::Int {
                base: NumberBase::Decimal,
                is_bigint: false,
                ..
            })
        )
    }

    /// Return true when a decimal integer uses member access without a separator.
    #[inline]
    pub(super) fn invalid_decimal_integer_member_access(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        distance: u8,
    ) -> bool {
        // only direct `.name` member access needs the separator rule
        if distance != 2 {
            return false;
        }

        // reject direct member access when the receiver is a decimal integer
        let dot_index = self.pos_index().saturating_sub(1);
        self.expression_is_decimal_integer_before_dot(left_expression_id, dot_index)
    }

    /// Return true when a type expression is a decimal integer token directly before `.`.
    #[inline]
    pub(super) fn type_is_decimal_integer_before_dot(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
        dot_index: usize,
    ) -> bool {
        // only integer scalar literals can use decimal separators for member access
        if !matches!(
            self.tree.get(left_type_id),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(_),
            }
        ) {
            return false;
        }

        // the candidate must be a dot token adjacent to the literal span
        let left_span = self.tree.get_span(left_type_id);
        let Some(dot_token) = self.token_ref_at(dot_index).copied() else {
            return false;
        };
        if dot_token.token.ty != TokenType::Dot || left_span.end != dot_token.span.start {
            return false;
        }

        // the left expression must map to one decimal integer literal token
        let Some(literal_index) = dot_index.checked_sub(1) else {
            return false;
        };
        let Some(literal_token) = self.token_ref_at(literal_index).copied() else {
            return false;
        };
        if literal_token.span != left_span || literal_token.token.ty != TokenType::Literal {
            return false;
        }

        matches!(
            literal_token.token.literal,
            Some(LiteralType::Int {
                base: NumberBase::Decimal,
                is_bigint: false,
                ..
            })
        )
    }

    /// Return true when a decimal integer type uses member access without a separator.
    #[inline]
    pub(super) fn invalid_decimal_integer_type_member_access(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
        distance: u8,
    ) -> bool {
        // only direct `.name` member access needs the separator rule
        if distance != 2 {
            return false;
        }

        // reject direct member access when the receiver is a decimal integer
        let dot_index = self.pos_index().saturating_sub(1);
        self.type_is_decimal_integer_before_dot(left_type_id, dot_index)
    }

    /// Eat a static member name and return both the name and its span.
    #[inline]
    pub(super) fn eat_member_name_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        // identifier member name
        if self.peek_is(TokenType::Identifier) {
            self.eat_identifier_with_span()
        }
        // boolean literal member name
        else if self.peek_is(TokenType::Literal)
            && self
                .peek()
                .is_ok_and(|token| matches!(token.token.literal, Some(LiteralType::Boolean { .. })))
        {
            let token = *self.eat()?;
            let text = self.get_token_str(token).to_owned();
            let name = self.strings.intern(&text);
            Ok((name, token.span))
        }
        // invalid member name
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Check whether `?.` starts an optional chaining segment.
    #[inline]
    pub(super) fn is_optional_chain_after_maybe(&mut self) -> bool {
        self.is_optional_chain_after_maybe_at(self.pos_index())
    }

    /// Check whether `?.` starts an optional chaining segment at a token index.
    #[inline]
    pub(super) fn is_optional_chain_after_maybe_at(&mut self, maybe_index: usize) -> bool {
        // require ?. before we look at the target
        if self.token_type_at(maybe_index.saturating_add(1)) != TokenType::Dot {
            return false;
        }

        // accept valid optional chain targets after ?., including line-delimited targets
        let next_target_index = self.first_non_newline_index_from(maybe_index.saturating_add(2));
        let next_target_type = self.token_type_at(next_target_index);
        matches!(
            next_target_type,
            TokenType::Identifier
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::Hash
                | TokenType::LessThan
                | TokenType::ShiftLeft
                | TokenType::TemplateStringStart
                | TokenType::TemplateString
        ) || Self::is_expression_slot_boundary_token(next_target_type)
    }

    /// Check whether `asserts` starts a type predicate.
    #[inline]
    pub(super) fn can_start_type_predicate_asserts(&mut self) -> bool {
        if !self.is_keyword(Keyword::Asserts) {
            return false;
        }

        let next_index = self.first_non_newline_index_from(self.pos_index().saturating_add(1));
        if self.keyword_for_index(next_index) == Some(Keyword::This) {
            return true;
        }

        self.token_type_at(next_index) == TokenType::Identifier
    }

    /// Eat an expression that might be parenthesized.
    pub fn eat_expression_parenthesized_maybe(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        if self.peek_is(TokenType::OpenParenthesis) {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let expression_id = self.eat_expression(self.options)?;
            self.eat_newlines_maybe()?;

            self.eat_close_token_or_recover_missing(
                TokenType::CloseParenthesis,
                NodeType::Expression,
            )?;
            Ok(expression_id)
        } else {
            self.eat_expression(self.options)
        }
    }
}
