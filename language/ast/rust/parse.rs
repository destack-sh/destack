use destack_language_token::{SourceFile, Span, Token, TokenSpan, TokenType};

use crate::{NodeTree, ParseError, ParseResult, StringPool};

const DEFAULT_EOF_TOKEN_SPAN: TokenSpan = TokenSpan {
    span: Span { start: 0, end: 0 },
    token: Token::eof(),
};

/// A parser for the Destack Language.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored.
#[derive(Debug)]
pub struct Parser<'a> {
    // source
    /// The file we're parsing.
    pub(crate) file: SourceFile<'a>,
    /// The tokens to parse.
    pub(crate) tokens: &'a [TokenSpan],
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    eof_token: TokenSpan,

    // ast
    /// The string pool.
    pub(crate) strings: StringPool,
    /// The Node tree.
    pub(crate) tree: NodeTree,
    
    // state
    /// The current position in the tokens.
    pos: usize,
    
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(file: SourceFile<'a>, tokens: &'a [TokenSpan]) -> Self {
        let eof_token = *tokens.last().unwrap_or(&DEFAULT_EOF_TOKEN_SPAN);
        let strings = StringPool::new();
        let tree = NodeTree::new();
        Self {
            file,
            tokens,
            strings,
            tree,
            pos: 0,
            eof_token,
        }
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark { pos: self.pos }
    }

    /// Get a mark and return the span of the current position.
    #[inline]
    pub fn get_mark_span(&self, mark: ParserMark) -> Span {
        let start_token = self.tokens[mark.pos];
        let end_token = self.tokens[self.pos];
        let span = Span {
            start: start_token.span.start,
            end: end_token.span.end,
        };
        span
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
        self.tokens
            .get(self.pos)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 1)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
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

    /// Eat a semicolon.
    #[inline]
    pub fn eat_semicolon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&self) -> ParseResult<&TokenSpan> {
        self.peek_next_token(TokenType::Semicolon)
    }

    /// Eat a newline.
    #[inline]
    pub fn eat_newline(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Newline)
    }

    /// Peek a newline.
    #[inline]
    pub fn peek_newline(&self) -> ParseResult<&TokenSpan> {
        self.peek_next_token(TokenType::Newline)
    }

    /// Eat a "stop" (semicolon or newline).
    #[inline]
    pub fn eat_stop(&mut self) -> ParseResult<&TokenSpan> {
        if self.peek_newline().is_ok() {
            self.eat_newline()
        } else {
            self.eat_semicolon()
        }
    }

    /// Peek a "stop" (semicolon or newline).
    #[inline]
    pub fn peek_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(newline) = self.peek_newline() {
            Ok(newline)
        } else {
            self.peek_semicolon()
        }
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct ParserMark {
    /// The token position.
    pos: usize,
}