use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};
use destack_ast::{Keyword, TokenSpan, TokenType};

impl Parser {
    /// Return true when the token at index is the given keyword.
    #[inline]
    fn keyword_is_at(&mut self, index: usize, keyword: Keyword) -> bool {
        self.keyword_for_index(index) == Some(keyword)
    }

    /// Return true when the current token is the given keyword.
    #[inline]
    pub fn is_keyword(&mut self, keyword: Keyword) -> bool {
        let pos = self.pos_index();
        self.keyword_is_at(pos, keyword)
    }

    /// Return true when the next token is the given keyword.
    #[inline]
    pub fn is_next_keyword(&mut self, keyword: Keyword) -> bool {
        let pos = self.index_for_next();
        self.keyword_is_at(pos, keyword)
    }

    /// Return true when the next next token is the given keyword.
    #[inline]
    pub fn is_next_next_keyword(&mut self, keyword: Keyword) -> bool {
        let pos = self.index_for_next_next();
        self.keyword_is_at(pos, keyword)
    }

    /// Return true when the token after any leading newlines is the given keyword.
    #[inline]
    pub fn is_keyword_after_newlines(&mut self, keyword: Keyword) -> bool {
        let cursor = self.peek_scanner_cursor();
        self.keyword_is_at(cursor.index, keyword)
    }

    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = *self.peek_token(TokenType::Identifier)?;
        if !self.is_keyword(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            self.peek_token(TokenType::Identifier)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&mut self) -> ParseResult<Keyword> {
        let current = *self.peek_token(TokenType::Identifier)?;
        self.keyword_for_index(self.pos_index())
            .ok_or_else(|| ParseError::expected(current.span, TokenType::Identifier))
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = *self.peek_next_token(TokenType::Identifier)?;
        if !self.is_next_keyword(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            self.peek_next_token(TokenType::Identifier)
        }
    }

    /// Peek any next keyword.
    #[inline]
    pub fn peek_next_any_keyword(&mut self) -> ParseResult<Keyword> {
        let current = *self.peek_next_token(TokenType::Identifier)?;
        let pos = self.index_for_next();
        self.keyword_for_index(pos)
            .ok_or_else(|| ParseError::expected(current.span, TokenType::Identifier))
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = *self.peek_next_next_token(TokenType::Identifier)?;
        let pos = self.index_for_next_next();
        if self.keyword_for_index(pos) != Some(keyword) {
            Err(ParseError::expected(current.span, TokenType::Identifier))
        } else {
            self.peek_next_next_token(TokenType::Identifier)
        }
    }

    /// Peek a keyword after any newlines.
    #[inline]
    pub fn peek_keyword_after_newlines(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let pos = self.pos_index();
        self.ensure_token(pos);
        let pos = if self
            .tokens()
            .get(pos)
            .is_some_and(|token| token.token.ty != TokenType::Newline)
        {
            pos
        } else {
            self.next_non_newline_index_from_stream(pos)
        };
        let matches_keyword = self.is_keyword_after_newlines(keyword);
        let eof_span = self.eof_span();
        let current = self
            .token_ref_at(pos)
            .ok_or_else(|| ParseError::expected(eof_span, TokenType::Identifier))?;
        if matches_keyword {
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
