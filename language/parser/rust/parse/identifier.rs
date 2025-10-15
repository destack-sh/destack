//! Parse identifiers.

use crate::{Parser, ParserError, ParserResult, TokenSpan, TokenType};
use dyst_source::StringId;

impl<'a> Parser<'a> {
    /// Peek an identifier.
    #[inline]
    pub fn peek_identifier(&self) -> ParserResult<&TokenSpan> {
        self.peek_token(TokenType::Identifier)
    }

    /// Eat an identifier.
    #[inline]
    pub fn eat_identifier(&mut self) -> ParserResult<StringId> {
        let token = *self.eat_token(TokenType::Identifier)?;
        let string_id = self.intern_string(self.get_token_str(token));
        Ok(string_id)
    }

    /// Peek an identifier that matches a given string.
    #[inline]
    pub fn peek_identifier_str(&self, string: &str) -> ParserResult<&TokenSpan> {
        let span = self.peek_token(TokenType::Identifier)?;
        if self.get_token_str(*span) == string {
            Ok(span)
        } else {
            Err(ParserError::expected(span.span, TokenType::Identifier))
        }
    }

    /// Eat an identifier that matches a given string.
    #[inline]
    pub fn eat_identifier_str(&mut self, string: &str) -> ParserResult<StringId> {
        let span = self.peek_identifier_str(string)?;
        let string_id = self.intern_string(self.get_token_str(*span));
        self.bump();
        Ok(string_id)
    }

    /// Eat an identifier or a wildcard maybe.
    #[inline]
    pub fn eat_identifier_or_wildcard_maybe(&mut self) -> ParserResult<Option<StringId>> {
        if self.peek_token(TokenType::Wildcard).is_ok() {
            self.bump();
            Ok(None)
        } else if self.peek_identifier().is_ok() {
            Ok(Some(self.eat_identifier()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a tree literal identifier (`kebab-case` as `kebabCase`).
    #[inline]
    pub fn eat_tree_literal_identifier(&mut self) -> ParserResult<StringId> {
        let mut identifier = String::new();
        loop {
            let token = *self.eat_token(TokenType::Identifier)?;
            let token_part = self.get_token_str(token);

            // uppercase first letter (except at start)
            if identifier.is_empty() {
                identifier.push_str(token_part);
            } else {
                identifier.push_str(&token_part[0..1].to_uppercase());
                identifier.push_str(&token_part[1..]);
            }

            if self.peek_token(TokenType::Subtract).is_ok() {
                self.bump();
            } else {
                break;
            }
        }
        let string_id = self.intern_string(identifier);
        Ok(string_id)
    }
}
