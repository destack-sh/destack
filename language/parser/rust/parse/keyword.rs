use std::str::FromStr;

use crate::{AstResult, Keyword, Parser, ParserError, TokenSpan, TokenType};

impl<'a> Parser<'a> {
    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> AstResult<&TokenSpan> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParserError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&self) -> AstResult<Keyword> {
        let current = self.peek_token(TokenType::Identifier)?;
        if let Ok(keyword) = Keyword::from_str(self.get_span_str(current.span)) {
            Ok(keyword)
        } else {
            Err(ParserError::expected(current.span, TokenType::Identifier))
        }
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&self, keyword: Keyword) -> AstResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParserError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&self, keyword: Keyword) -> AstResult<&TokenSpan> {
        let current = self.peek_next_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParserError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> AstResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }
}
