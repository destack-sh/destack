use crate::{Parser, ParserError, ParserResult};
use tspp_dir::{Keyword, Token, TokenType};

impl Parser {
    /// Return true when the current token is the given keyword.
    #[inline]
    pub fn peek_is_keyword(&self, keyword: Keyword) -> bool {
        self.peek_keyword() == Some(keyword)
    }

    /// Return the current token when it is the requested keyword.
    #[inline]
    pub fn peek_keyword_token(&self, keyword: Keyword) -> ParserResult<Token> {
        let current = self.require_token(TokenType::Identifier)?;
        if !self.peek_is_keyword(keyword) {
            Err(ParserError::expected(current, TokenType::Identifier))
        } else {
            Ok(current)
        }
    }

    /// Peek any keyword.
    #[inline]
    pub fn peek_any_keyword(&self) -> ParserResult<Keyword> {
        let current = self.require_token(TokenType::Identifier)?;
        self.peek_keyword()
            .ok_or_else(|| ParserError::expected(current, TokenType::Identifier))
    }

    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParserResult<Token> {
        self.peek_keyword_token(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Eat one of a list of keywords.
    pub fn eat_keyword_in(&mut self, keywords: &[Keyword]) -> ParserResult<Keyword> {
        let keyword = self.peek_any_keyword()?;
        if keywords.contains(&keyword) {
            self.eat_keyword(keyword)?;
            Ok(keyword)
        } else {
            Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::Identifier,
            ))
        }
    }
}
