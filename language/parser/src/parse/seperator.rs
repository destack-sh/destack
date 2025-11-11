use crate::{Parser, ParserError, ParserResult};
use dyst_ast::{TokenSpan, TokenType};

impl<'a> Parser<'a> {
    /// Peek a colon.
    #[inline]
    pub fn peek_colon(&self) -> ParserResult<&TokenSpan> {
        self.peek_token(TokenType::Colon)
    }

    /// Eat a colon.
    #[inline]
    pub fn eat_colon(&mut self) -> ParserResult<&TokenSpan> {
        self.eat_token(TokenType::Colon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&self) -> ParserResult<&TokenSpan> {
        self.peek_token(TokenType::Semicolon)
    }

    /// Eat a semicolon.
    #[inline]
    pub fn eat_semicolon(&mut self) -> ParserResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a comma.
    #[inline]
    pub fn peek_comma(&self) -> ParserResult<&TokenSpan> {
        self.peek_token(TokenType::Comma)
    }

    /// Eat a comma.
    #[inline]
    pub fn eat_comma(&mut self) -> ParserResult<&TokenSpan> {
        self.eat_token(TokenType::Comma)
    }

    /// Peek a newline.
    #[inline]
    pub fn peek_newline(&self) -> ParserResult<&TokenSpan> {
        self.peek_token(TokenType::Newline)
    }

    /// Eat a newline.
    #[inline]
    pub fn eat_newline(&mut self) -> ParserResult<&TokenSpan> {
        self.eat_token(TokenType::Newline)
    }

    /// Eat 0 or 1 newline.
    #[inline]
    pub fn eat_newline_maybe(&mut self) -> ParserResult<()> {
        let token = self.peek()?;
        if token.token.ty == TokenType::Newline {
            self.bump();
        }
        Ok(())
    }

    /// Eat 0 or more newlines.
    #[inline]
    pub fn eat_newlines_maybe(&mut self) -> ParserResult<()> {
        while self.peek_newline().is_ok() {
            self.eat_newline()?;
        }
        Ok(())
    }

    /// Peek an arrow.
    #[inline]
    pub fn peek_arrow(&self) -> ParserResult<&TokenSpan> {
        let token = self.peek()?;
        if token.token.ty == TokenType::ArrowWide || token.token.ty == TokenType::Arrow {
            Ok(token)
        } else {
            Err(ParserError::expected(token.span, TokenType::ArrowWide))
        }
    }

    /// Eat an arrow.
    #[inline]
    pub fn eat_arrow(&mut self) -> ParserResult<&TokenSpan> {
        let token = self.eat()?;
        if token.token.ty == TokenType::ArrowWide || token.token.ty == TokenType::Arrow {
            Ok(token)
        } else {
            Err(ParserError::expected(token.span, TokenType::ArrowWide))
        }
    }
}
