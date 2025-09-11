//! Parse identifiers.

use dyst_language_arena::StringId;
use dyst_language_token::{TokenSpan, TokenType};

use crate::{ParseError, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParseResult<StringId> {
        let token = *self.eat_token(TokenType::Identifier)?;
        let string_id = self.strings.intern(self.get_token_str(token));
        Ok(string_id)
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&self, string: &str) -> ParseResult<&TokenSpan> {
        let span = self.peek_token(TokenType::Identifier)?;
        if self.get_token_str(*span) == string {
            Ok(span)
        } else {
            Err(ParseError::expected_token(span.span, TokenType::Identifier))
        }
    }

    /// Eat an identifier that matches a given string.
    #[inline]
    pub fn eat_identifier_str(&mut self, string: &str) -> ParseResult<StringId> {
        let span = self.peek_identifier_str(string)?;
        let string_id = self.strings.intern(self.get_token_str(*span));
        self.bump();
        Ok(string_id)
    }

    /// Eat an identifier or a wildcard maybe.
    #[inline]
    pub fn eat_identifier_or_wildcard_maybe(&mut self) -> ParseResult<Option<StringId>> {
        if self.peek_token(TokenType::Wildcard).is_ok() {
            self.bump();
            Ok(None)
        } else if self.peek_identifier().is_ok() {
            Ok(Some(self.eat_identifier()?))
        } else {
            Ok(None)
        }
    }
}
