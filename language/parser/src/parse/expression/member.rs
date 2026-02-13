use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Expression, Keyword, LiteralType, LocalNodeId, TokenType};
use destack_base::StringId;

impl Parser {
    /// Return true when a decimal integer uses member access without a separator.
    #[inline]
    pub(super) fn invalid_decimal_integer_member_access(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        distance: u8,
    ) -> bool {
        if distance != 2 {
            return false;
        }

        if !matches!(
            self.tree.get(left_expression_id),
            Expression::ScalarLiteral(destack_ast::ScalarLiteral::Integer(_))
        ) {
            return false;
        }

        let left_span = self.tree.get_span(left_expression_id);
        let Some(dot_token) = self.prev().copied() else {
            return false;
        };
        if dot_token.token.ty != TokenType::Dot {
            return false;
        }
        if left_span.end != dot_token.span.start {
            return false;
        }

        let literal = self.file.span_str(left_span);
        let is_non_decimal_prefix = literal.starts_with("0x")
            || literal.starts_with("0X")
            || literal.starts_with("0o")
            || literal.starts_with("0O")
            || literal.starts_with("0b")
            || literal.starts_with("0B");
        if is_non_decimal_prefix {
            return false;
        }

        true
    }

    /// Eat a static member name and return both the name and its span.
    #[inline]
    pub(super) fn eat_member_name_with_span(
        &mut self,
    ) -> ParseResult<(StringId, destack_source::Span)> {
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

        // accept valid optional chain targets after ?.
        let next_next_token_type = self.token_type_at(maybe_index.saturating_add(2));
        matches!(
            next_next_token_type,
            TokenType::Identifier
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::Hash
                | TokenType::LessThan
                | TokenType::ShiftLeft
                | TokenType::TemplateStringStart
                | TokenType::TemplateString
        )
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
        let start = self.mark_span();
        if self.peek_is(TokenType::OpenParenthesis) {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let expression_id = self.eat_expression(self.options)?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            self.tree
                .set_span(expression_id, self.get_span_from(&start));
            Ok(expression_id)
        } else {
            self.eat_expression(self.options)
        }
    }
}
