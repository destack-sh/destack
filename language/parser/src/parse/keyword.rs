use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};
use dyst_ast::{Keyword, TokenSpan, TokenType};

impl Parser {
    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&self) -> ParseResult<Keyword> {
        let current = self.peek_token(TokenType::Identifier)?;
        if let Ok(keyword) = Keyword::from_str(self.get_span_str(current.span)) {
            Ok(keyword)
        } else {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        }
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek a keyword after any newlines.
    #[inline]
    pub fn peek_keyword_after_newlines(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let mut pos = self.pos();
        while let Some(token) = self.tokens.get(pos as usize)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }
        let current = self.tokens.get(pos as usize).unwrap();
        if keyword.as_str() == self.get_span_str(current.span) {
            Ok(current)
        } else {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        }
    }

    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Eat any keyword.
    pub fn eat_keyword_any(&mut self) -> ParseResult<Keyword> {
        let current = *self.eat_token(TokenType::Identifier)?;
        let current_str = self.get_token_str(current);
        if let Ok(keyword) = Keyword::from_str(current_str) {
            Ok(keyword)
        } else {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        }
    }

    /// Eat one of a list of keywords.
    pub fn eat_keyword_in(&mut self, keywords: &[Keyword]) -> ParseResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keywords.contains(&keyword) {
            self.eat_keyword(keyword)?;
            Ok(keyword)
        } else {
            Err(ParseError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }
}
