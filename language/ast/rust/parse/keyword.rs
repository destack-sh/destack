use dyst_language_token::{TokenSpan, TokenType};

use crate::{Keyword, ParseError, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }
}
