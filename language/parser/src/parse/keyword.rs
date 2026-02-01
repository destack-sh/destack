use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};
use destack_ast::{Keyword, TokenSpan, TokenType};

impl Parser {
    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.has_active_split() {
            if self.get_span_str(current.span) != keyword.as_str() {
                return Err(ParseError::expected(current.span, TokenType::Identifier));
            }
            return Ok(current);
        }
        if self.keyword_for_index(self.pos_index()) != Some(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&self) -> ParseResult<Keyword> {
        let current = self.peek_token(TokenType::Identifier)?;
        if self.has_active_split() {
            return Keyword::from_str(self.get_span_str(current.span))
                .map_err(|_| ParseError::expected(current.span, TokenType::Identifier));
        }
        self.keyword_for_index(self.pos_index())
            .ok_or_else(|| ParseError::expected(current.span, TokenType::Identifier))
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        let pos = self.index_for_next();
        if self.keyword_for_index(pos) != Some(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek any next keyword.
    #[inline]
    pub fn peek_next_any_keyword(&self) -> ParseResult<Keyword> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        let pos = self.index_for_next();
        self.keyword_for_index(pos)
            .ok_or_else(|| ParseError::expected(current.span, TokenType::Identifier))
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_next_token(TokenType::Identifier)?;
        let pos = self.index_for_next_next();
        if self.keyword_for_index(pos) != Some(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek a keyword after any newlines.
    #[inline]
    pub fn peek_keyword_after_newlines(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let pos = self.pos_index();
        let len = self.tokens.len();
        let pos = if self
            .tokens
            .get(pos)
            .is_some_and(|token| token.token.ty != TokenType::Newline)
        {
            pos
        } else {
            self.next_non_newline
                .get(pos)
                .copied()
                .unwrap_or(len as u32) as usize
        };
        let current = self
            .tokens
            .get(pos)
            .ok_or_else(|| ParseError::expected(self.eof_token.span, TokenType::Identifier))?;
        if self.keyword_for_index(pos) == Some(keyword) {
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
        Keyword::from_str(self.get_token_str(current))
            .map_err(|_| ParseError::expected(current.span, TokenType::Identifier))
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
