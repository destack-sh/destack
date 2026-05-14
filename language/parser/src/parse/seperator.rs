use crate::{ParseError, ParseResult, Parser};
use destack_dir::{TokenSpan, TokenType};

impl Parser {
    /// Peek a colon.
    #[inline]
    pub fn peek_colon(&mut self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Colon)
    }

    /// Return true when the next token is a colon.
    #[inline]
    pub fn peek_colon_is(&mut self) -> bool {
        self.peek_is(TokenType::Colon)
    }

    /// Eat a colon.
    #[inline]
    pub fn eat_colon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Colon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&mut self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Semicolon)
    }

    /// Eat a semicolon.
    #[inline]
    pub fn eat_semicolon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a comma.
    #[inline]
    pub fn peek_comma(&mut self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Comma)
    }

    /// Return true when the next token is a comma.
    #[inline]
    pub fn peek_comma_is(&mut self) -> bool {
        self.peek_is(TokenType::Comma)
    }

    /// Eat a comma.
    #[inline]
    pub fn eat_comma(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Comma)
    }

    /// Peek an arrow.
    #[inline]
    pub fn peek_arrow(&mut self) -> ParseResult<&TokenSpan> {
        let token = self.peek()?;
        if token.token.ty == TokenType::ArrowWide || token.token.ty == TokenType::Arrow {
            Ok(token)
        } else {
            Err(ParseError::expected(token.span, TokenType::ArrowWide))
        }
    }

    /// Return true when the next token is an arrow.
    #[inline]
    pub fn peek_arrow_is(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::ArrowWide | TokenType::Arrow
        )
    }

    /// Eat an arrow.
    #[inline]
    pub fn eat_arrow(&mut self) -> ParseResult<&TokenSpan> {
        let token = self.eat()?;
        if token.token.ty == TokenType::ArrowWide || token.token.ty == TokenType::Arrow {
            Ok(token)
        } else {
            Err(ParseError::expected(token.span, TokenType::ArrowWide))
        }
    }
}
