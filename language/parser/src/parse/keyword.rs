use crate::{Parser, ParserError, ParserResult, keyword_from_identifier};
use destack_dir::{Keyword, TokenSpan, TokenType};

impl Parser {
    /// Return true when the current token is the given keyword.
    #[inline]
    pub fn is_keyword(&mut self, keyword: Keyword) -> bool {
        self.current_keyword() == Some(keyword)
    }

    /// Return true when the next token is the given keyword.
    #[inline]
    pub fn is_next_keyword(&mut self, keyword: Keyword) -> bool {
        self.keyword_at_offset(1) == Some(keyword)
    }

    /// Return true when the next next token is the given keyword.
    #[inline]
    pub fn is_next_next_keyword(&mut self, keyword: Keyword) -> bool {
        self.keyword_at_offset(2) == Some(keyword)
    }

    /// Peek a keyword.
    #[inline]
    pub fn peek_keyword(&mut self, keyword: Keyword) -> ParserResult<&TokenSpan> {
        let current = *self.peek_token(TokenType::Identifier)?;
        if !self.is_keyword(keyword) {
            Err(ParserError::expected(current.span, TokenType::Identifier))
        } else {
            self.peek_token(TokenType::Identifier)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&mut self) -> ParserResult<Keyword> {
        let current = *self.peek_token(TokenType::Identifier)?;
        self.current_keyword()
            .ok_or_else(|| ParserError::expected(current.span, TokenType::Identifier))
    }

    /// Peek the next keyword.
    #[inline]
    pub fn peek_next_keyword(&mut self, keyword: Keyword) -> ParserResult<TokenSpan> {
        let token = self.next_token();
        if token.token.ty() != TokenType::Identifier || self.keyword_at_offset(1) != Some(keyword) {
            return Err(ParserError::expected(token.span, TokenType::Identifier));
        }

        Ok(token)
    }

    /// Peek any next keyword.
    #[inline]
    pub fn peek_next_any_keyword(&mut self) -> ParserResult<Keyword> {
        let token = self.next_token();
        if token.token.ty() != TokenType::Identifier {
            return Err(ParserError::expected(token.span, TokenType::Identifier));
        }

        self.keyword_at_offset(1)
            .ok_or_else(|| ParserError::expected(token.span, TokenType::Identifier))
    }

    /// Peek the next next keyword.
    #[inline]
    pub fn peek_next_next_keyword(&mut self, keyword: Keyword) -> ParserResult<TokenSpan> {
        let token = self.token_at_offset(2);
        if token.token.ty() != TokenType::Identifier || self.keyword_at_offset(2) != Some(keyword) {
            return Err(ParserError::expected(token.span, TokenType::Identifier));
        }

        Ok(token)
    }

    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParserResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Eat any keyword.
    pub fn eat_keyword_any(&mut self) -> ParserResult<Keyword> {
        let current = *self.eat_token(TokenType::Identifier)?;
        keyword_from_identifier(self.get_token_str(current))
            .ok_or_else(|| ParserError::expected(current.span, TokenType::Identifier))
    }

    /// Eat one of a list of keywords.
    pub fn eat_keyword_in(&mut self, keywords: &[Keyword]) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keywords.contains(&keyword) {
            self.eat_keyword(keyword)?;
            Ok(keyword)
        } else {
            Err(ParserError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }
}
