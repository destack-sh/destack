use destack_language_lexer::{SourceFile, Span, Token, TokenSpan, TokenType};

use crate::{ParseError, ParseResult};

const DEFAULT_EOF_TOKEN_SPAN: TokenSpan = TokenSpan {
    span: Span { start: 0, end: 0 },
    token: Token::eof(),
};

/// A parser for the Destack Language.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Parser<'a> {
    /// The file we're parsing.
    pub(crate) file: SourceFile<'a>,
    /// The tokens to parse.
    pub(crate) tokens: &'a [TokenSpan],
    /// The current position in the tokens.
    pos: usize,
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    eof_token: TokenSpan,
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(file: SourceFile<'a>, tokens: &'a [TokenSpan]) -> Self {
        let eof_token = *tokens.last().unwrap_or(&DEFAULT_EOF_TOKEN_SPAN);
        Self {
            file,
            tokens,
            pos: 0,
            eof_token,
        }
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        &self.file.content[span.start as usize..span.end as usize]
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
        &self.file.content[token.span.start as usize..token.span.end as usize]
    }

    /// Get the current Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        self.tokens.get(self.pos)
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat_next(&mut self) -> ParseResult<&TokenSpan> {
        if self.pos < self.tokens.len() {
            let next = &self.tokens[self.pos];
            self.pos += 1;
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(self.eof_token.span))
        }
    }

    /// Bump the position.
    #[inline]
    pub fn bump(&mut self) {
        self.pos += 1;
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens.get(self.pos).ok_or(ParseError::SyntaxError(
            self.tokens.last().map(|s| s.span).unwrap(),
        ))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens.get(self.pos + 1).ok_or(ParseError::SyntaxError(
            self.tokens.last().map(|s| s.span).unwrap(),
        ))
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let current = self.eat_next()?;
        if current.token.r#type == token_type {
            Ok(current)
        } else {
            Err(ParseError::SyntaxError(current.span))
        }
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Eat a "stop" (semicolon or newline).
    #[inline]
    pub fn eat_stop(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&self) -> ParseResult<&TokenSpan> {
        self.peek_next_token(TokenType::Semicolon)
    }

    /// Eat a colon.
    #[inline]
    pub fn eat_colon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Colon)
    }

    /// Peek a colon.
    #[inline]
    pub fn peek_colon(&self) -> ParseResult<&TokenSpan> {
        self.peek_next_token(TokenType::Colon)
    }
}
