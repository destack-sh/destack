use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Expression, Keyword, LiteralType, LocalNodeId, TokenSpan, TokenType};
use destack_base::StringId;

impl Parser {
    /// Convert a computed token distance to `u8`.
    #[inline]
    fn member_distance(distance: usize) -> u8 {
        u8::try_from(distance).unwrap_or(u8::MAX)
    }

    /// Scan newlines from `base + start_offset` and return `(offset, count)`.
    #[inline]
    fn scan_newlines_from(&mut self, base: usize, start_offset: usize) -> (usize, usize) {
        let mut offset = start_offset;
        let mut newline_count = 0;
        loop {
            self.ensure_token(base + offset);
            let Some(token) = self.tokens().get(base + offset) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            newline_count += 1;
            offset += 1;
        }

        (offset, newline_count)
    }

    /// Return true when the token at an absolute index has the requested type.
    #[inline]
    fn token_type_at_index_is(&mut self, index: usize, token_type: TokenType) -> bool {
        self.ensure_token(index);
        self.tokens()
            .get(index)
            .is_some_and(|token| token.token.ty == token_type)
    }

    pub(super) fn token_is_member_name(token: &TokenSpan) -> bool {
        token.token.ty == TokenType::Identifier
            || matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
    }

    /// Return true when the token at an absolute index is a valid member name.
    #[inline]
    fn token_is_member_name_at_index(&mut self, index: usize) -> bool {
        self.ensure_token(index);
        self.tokens()
            .get(index)
            .is_some_and(Self::token_is_member_name)
    }

    /// Return the member distance for `.<name>` from an offset relative to `base`.
    #[inline]
    fn peek_member_distance_from_offset(&mut self, base: usize, start_offset: usize) -> Option<u8> {
        // require `.`
        if !self.token_type_at_index_is(base + start_offset, TokenType::Dot) {
            return None;
        }

        // allow newlines after `.`
        let (name_offset, _) = self.scan_newlines_from(base, start_offset + 1);
        if !self.token_is_member_name_at_index(base + name_offset) {
            return None;
        }

        Some(Self::member_distance(name_offset + 1))
    }

    /// Return the private-member distance for `.#name` from an offset relative to `base`.
    #[inline]
    fn peek_private_member_distance_from_offset(
        &mut self,
        base: usize,
        start_offset: usize,
    ) -> ParseResult<Option<u8>> {
        // require `.`
        if !self.token_type_at_index_is(base + start_offset, TokenType::Dot) {
            return Ok(None);
        }

        // allow newlines after `.`
        let (hash_offset, _) = self.scan_newlines_from(base, start_offset + 1);
        let hash_index = base + hash_offset;
        if !self.token_type_at_index_is(hash_index, TokenType::Hash) {
            return Ok(None);
        }

        // require `#` + identifier with no trivia between them
        let identifier_index = hash_index + 1;
        if !self.token_type_at_index_is(identifier_index, TokenType::Identifier) {
            return Ok(None);
        }
        self.check_tokens_are_adjacent(hash_index, identifier_index)?;

        // include all prefix tokens plus `#` + identifier
        let distance = identifier_index - base + 1;
        Ok(Some(Self::member_distance(distance)))
    }

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

    /// Peek a member access with an IdentifierName compatible token.
    /// Returns the total distance to eat (including the newlines, dot, and token).
    #[inline]
    pub(super) fn peek_member_name_maybe(&mut self) -> Option<u8> {
        let base = self.pos_index();

        // direct case: `.name` or `.\nname`
        if let Some(distance) = self.peek_member_distance_from_offset(base, 0) {
            return Some(distance);
        }

        // continuation case: `\n.name` or `\n.\nname`
        let (offset, newline_count) = self.scan_newlines_from(base, 0);
        if newline_count > 0
            && let Some(distance) = self.peek_member_distance_from_offset(base, offset)
        {
            return Some(distance);
        }

        None
    }

    /// Check whether `?.` starts an optional chaining segment.
    #[inline]
    pub(super) fn is_optional_chain_after_maybe(&mut self) -> bool {
        // require ?. before we look at the target
        if !self.peek_next_is(TokenType::Dot) {
            return false;
        }

        // accept valid optional chain targets after ?.
        let next_next_token_type = self.token_type_at(self.pos() as usize + 2);
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

    /// Return true when optional chaining starts after one or more newlines.
    #[inline]
    pub(super) fn optional_chain_starts_after_newlines(&mut self) -> bool {
        if !self.peek_is(TokenType::Newline) {
            return false;
        }

        let next_index = self.next_non_newline_index_from(self.pos_index());
        self.ensure_token(next_index + 1);
        let Some(next_token) = self.tokens().get(next_index) else {
            return false;
        };

        if next_token.token.ty == TokenType::Maybe {
            return true;
        }

        next_token.token.ty == TokenType::Dot
            && self
                .tokens()
                .get(next_index + 1)
                .is_some_and(|token| token.token.ty == TokenType::Maybe)
    }

    /// Check whether `asserts` starts a type predicate.
    #[inline]
    pub(super) fn can_start_type_predicate_asserts(&mut self) -> bool {
        if !self.is_keyword(Keyword::Asserts) {
            return false;
        }

        let mut pos = self.pos() as usize;
        loop {
            self.ensure_token(pos + 1);
            let Some(token) = self.tokens().get(pos + 1) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            pos += 1;
        }

        if self.keyword_for_index(pos + 1) == Some(Keyword::This) {
            return true;
        }

        self.tokens()
            .get(pos + 1)
            .is_some_and(|token| token.token.ty == TokenType::Identifier)
    }

    /// Peek a private member access using `.#`.
    #[inline]
    pub(super) fn peek_private_member_maybe(&mut self) -> ParseResult<Option<u8>> {
        let base = self.pos_index();

        // direct case: `.#name` or `.\n#name`
        if let Some(distance) = self.peek_private_member_distance_from_offset(base, 0)? {
            return Ok(Some(distance));
        }

        // continuation case: `\n.#name` or `\n.\n#name`
        let (offset, newline_count) = self.scan_newlines_from(base, 0);
        if newline_count > 0
            && let Some(distance) = self.peek_private_member_distance_from_offset(base, offset)?
        {
            return Ok(Some(distance));
        }

        Ok(None)
    }

    /// Eat an expression that might be parenthesized.
    pub fn eat_expression_parenthesized_maybe(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
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
