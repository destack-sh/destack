use crate::{ParserError, AstResult, Parser, TokenSpan, TokenType};

impl<'a> Parser<'a> {
    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&self) -> AstResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline || token.token.ty == TokenType::Comma)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ))
        }
    }

    /// Eat an item stop (comma or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_item_stop_with_newlines(&mut self) -> AstResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline || token.token.ty == TokenType::Comma)
        {
            self.bump();
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&self) -> AstResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat a statement stop (semicolon or newline).
    #[inline]
    pub fn eat_statement_stop(&mut self) -> AstResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        Ok(())
    }

    /// Eat a statement stop (semicolon or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_statement_stop_with_newlines(&mut self) -> AstResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_any_stop(&self) -> AstResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat any stop (comma, semicolon, or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_any_stop_with_newlines(&mut self) -> AstResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            self.bump();
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }
}
