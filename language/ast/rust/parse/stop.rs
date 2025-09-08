use dyst_language_token::{TokenSpan, TokenType};

use crate::{ParseError, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline || token.token.r#type == TokenType::Comma)
        {
            Ok(token)
        } else {
            Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ))
        }
    }

    /// Eat an item stop (comma or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_item_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline || token.token.r#type == TokenType::Comma)
        {
            self.bump();
        } else {
            return Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline
                || token.token.r#type == TokenType::Semicolon)
        {
            Ok(token)
        } else {
            Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ))
        }
    }

    /// Eat a statement stop (semicolon or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_statement_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline
                || token.token.r#type == TokenType::Semicolon)
        {
            self.bump();
        } else {
            return Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_any_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline
                || token.token.r#type == TokenType::Semicolon
                || token.token.r#type == TokenType::Comma)
        {
            Ok(token)
        } else {
            Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ))
        }
    }

    /// Eat any stop (comma, semicolon, or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_any_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.r#type == TokenType::Newline
                || token.token.r#type == TokenType::Semicolon
                || token.token.r#type == TokenType::Comma)
        {
            self.bump();
        } else {
            return Err(ParseError::SyntaxError(
                self.peek().unwrap_or(&self.eof_token).span,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }
}
