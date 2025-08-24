use destack_language_lexer::{SemanticToken, TokenType};

use crate::ParseError;

/// A parser for the Destack Language.
///
/// The Parser works on semantic undifferentiated Tokens (keywords are contextual).
/// Whitespace and regular line comments are ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Parser<'a> {
    /// The tokens to parse.
    pub(crate) tokens: &'a [SemanticToken],
    /// The current position in the tokens.
    pub(crate) pos: usize,
}

/// A result of a parse operation.
pub type ParseResult<'a, T> = Result<T, ParseError>;

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(tokens: &'a [SemanticToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Get the current token.
    #[inline]
    pub fn current(&self) -> Option<&SemanticToken> {
        self.tokens.get(self.pos)
    }

    /// Bump the position.
    pub fn bump(&mut self) {
        self.pos += 1;
    }

    /// Eat a token.
    pub fn eat_token(&mut self, token: TokenType) -> ParseResult<'a, ()> {
        todo!()
    }

    /// Peek a token.
    pub fn peek_token(&self, token: TokenType) -> ParseResult<'a, ()> {
        todo!()
    }

    /// Eat a semicolon.
    pub fn eat_semicolon(&mut self) -> ParseResult<'a, ()> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a semicolon.
    pub fn peek_semicolon(&self) -> ParseResult<'a, ()> {
        self.peek_token(TokenType::Semicolon)
    }
}
