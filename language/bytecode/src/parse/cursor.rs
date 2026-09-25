use tspp_source::FileId;

use crate::{Lexer, ParseError, ParseResult, Parser, Token, TokenType};

/// Cursor over one tokenized bytecode source.
#[derive(Debug)]
pub(super) struct TokenCursor<'a> {
    /// Source file identity.
    pub(super) file_id: FileId,
    /// Original source text.
    pub(super) source: &'a str,
    /// Lexical tokens including trivia.
    tokens: Vec<Token>,
    /// Current token index.
    position: usize,
}

impl<'a> TokenCursor<'a> {
    /// Create one cursor over tokenized bytecode source.
    pub(super) fn new(file_id: FileId, source: &'a str) -> Self {
        Self {
            file_id,
            source,
            tokens: Lexer::lex(file_id, source),
            position: 0,
        }
    }

    /// Return the next non-trivia token.
    fn peek(&mut self) -> Token {
        self.skip_trivia();

        self.tokens[self.position]
    }

    /// Advance over the next non-trivia token.
    fn bump(&mut self) -> Token {
        let token = self.peek();

        // retain the terminal position after reaching end of source
        if token.ty != TokenType::End {
            self.position += 1;
        }

        token
    }

    /// Return one token's exact source text.
    fn text(&self, token: Token) -> &'a str {
        &self.source[token.span.start as usize..token.span.end as usize]
    }

    /// Return the previous non-trivia token.
    fn previous(&self) -> Token {
        let mut position = self.position - 1;

        // walk back over trivia to the previous parser-visible token
        while self.tokens[position].is_trivia() {
            position -= 1;
        }

        self.tokens[position]
    }

    /// Return the current raw token position.
    pub(super) const fn position(&self) -> usize {
        self.position
    }

    /// Move to one raw token position.
    pub(super) fn seek(&mut self, position: usize) {
        self.position = position;
    }

    /// Skip whitespace, comments, and newlines.
    fn skip_trivia(&mut self) {
        while self.tokens[self.position].is_trivia() {
            self.position += 1;
        }
    }
}

impl<'source> Parser<'source> {
    /// Return the next non-trivia token.
    pub(super) fn peek(&mut self) -> Token {
        self.cursor.peek()
    }

    /// Advance over the next non-trivia token.
    pub(super) fn bump(&mut self) -> Token {
        self.cursor.bump()
    }

    /// Return one token's exact source text.
    pub(super) fn text(&self, token: Token) -> &'source str {
        self.cursor.text(token)
    }

    /// Return whether the next token is one exact identifier.
    pub(super) fn peek_name(&mut self, expected: &str) -> bool {
        let token = self.peek();

        token.ty == TokenType::Identifier && self.text(token) == expected
    }

    /// Return whether the next token has one category.
    pub(super) fn peek_is(&mut self, expected: TokenType) -> bool {
        self.peek().ty == expected
    }

    /// Consume one token category.
    pub(super) fn eat_token(&mut self, expected: TokenType) -> ParseResult<Token> {
        let token = self.bump();

        // reject any other parser-visible token category
        if token.ty != expected {
            return Err(ParseError::new(
                format!("expected {expected:?}"),
                token.span,
            ));
        }

        Ok(token)
    }

    /// Consume one exact identifier.
    pub(super) fn eat_name(&mut self, expected: &str) -> ParseResult<Token> {
        let token = self.eat_token(TokenType::Identifier)?;

        // reject any other identifier text
        if self.text(token) != expected {
            return Err(ParseError::new(
                format!("expected '{expected}'"),
                token.span,
            ));
        }

        Ok(token)
    }

    /// Consume one exact identifier when present.
    pub(super) fn eat_name_if(&mut self, expected: &str) -> bool {
        if !self.peek_name(expected) {
            return false;
        }
        self.bump();

        true
    }

    /// Consume one token category when present.
    pub(super) fn eat_token_if(&mut self, expected: TokenType) -> bool {
        if !self.peek_is(expected) {
            return false;
        }
        self.bump();

        true
    }

    /// Parse one unsigned 16-bit integer literal.
    pub(super) fn parse_u16(&mut self) -> ParseResult<u16> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.text(token).replace('_', "");

        text.parse()
            .map_err(|_| ParseError::new("expected unsigned 16-bit integer", token.span))
    }

    /// Parse one unsigned 32-bit integer literal.
    pub(super) fn parse_u32(&mut self) -> ParseResult<u32> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.text(token).replace('_', "");

        text.parse()
            .map_err(|_| ParseError::new("expected unsigned 32-bit integer", token.span))
    }

    /// Parse one unsigned 64-bit integer literal.
    pub(super) fn parse_u64(&mut self) -> ParseResult<u64> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.text(token).replace('_', "");

        text.parse()
            .map_err(|_| ParseError::new("expected unsigned 64-bit integer", token.span))
    }

    /// Parse one signed 64-bit integer literal.
    pub(super) fn parse_i64(&mut self) -> ParseResult<i64> {
        let token = self.eat_token(TokenType::Integer)?;
        let text = self.text(token).replace('_', "");

        text.parse()
            .map_err(|_| ParseError::new("expected signed 64-bit integer", token.span))
    }

    /// Return the previous non-trivia token.
    pub(super) fn previous(&self) -> Token {
        self.cursor.previous()
    }
}
