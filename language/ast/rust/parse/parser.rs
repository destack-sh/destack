use core::fmt;
use std::fmt::Debug;

use dyst_language_diagnostic::Diagnostic;
use dyst_language_source::{Source, SourceId, Span};
use dyst_language_token::{Token, TokenSpan, TokenType, tokenize_semantic};

use crate::{
    Dumper, DumperOptions, NodeTree, ParseError, ParseResult, PathPool, StringPool, Visibility,
};

/// Configure parsing behavior.
/// Useful for enabling/disabling features in some AST subtrees.
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct ParserOptions {
    /// The default visibility.
    pub default_visibility: Visibility = Visibility::Private,
    /// Whether we're parsing a static type (parameters or arguments).
    /// We disallow certain infix operations in static types to avoid ambiguity with <>.
    pub in_static_type: bool = false,
    /// Whether we're parsing an implicit union pattern.
    pub in_implicit_union: bool = false,
}

/// A parser for Dyst source code.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser<'a> {
    /// The source we're parsing.
    pub source: &'a Source,
    /// The source ID.
    pub source_id: SourceId,
    /// The tokens parsed from the source.
    pub tokens: Vec<TokenSpan>,
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    pub eof_token: TokenSpan,

    // todo!: Parser Session & :Diagnostics (with some recovery)
    /// Diagnostics emitted in this session.
    pub diagnostics: Vec<Diagnostic>,
    /// The string pool.
    pub strings: StringPool,
    /// The path pool.
    pub paths: PathPool,
    /// The Node tree.
    pub tree: NodeTree,

    /// The current position in the tokens.
    pos: usize,
    /// The parser options.
    pub(crate) options: ParserOptions,
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn from_source(source: &'a Source, source_id: SourceId) -> Self {
        let tokens = tokenize_semantic(source_id, &source.content);
        let eof_token = *tokens.last().unwrap_or(&TokenSpan {
            span: Span {
                source: source_id,
                start: 0,
                end: 0,
            },
            token: Token::eof(),
        });
        let strings = StringPool::new();
        let paths = PathPool::new();
        let tree = NodeTree::new();
        let diagnostics = Vec::new();
        Self {
            source,
            source_id,
            tokens,
            strings,
            paths,
            tree,
            diagnostics,
            pos: 0,
            eof_token,
            options: ParserOptions::default(),
        }
    }

    /// Create a new Dumper.
    pub fn dumper(&self, options: DumperOptions) -> Dumper<'_> {
        Dumper::new(&self.strings, &self.paths, &self.tree, options)
    }

    /// Get the current position.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Execute a function with a new parser options.
    /// The old options are restored after the function returns.
    #[inline]
    pub(crate) fn with_options<T>(
        &mut self,
        options: ParserOptions,
        func: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let old_options = self.options;
        self.options = options;
        let result = func(self);
        self.options = old_options;
        result
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        debug_assert!(self.pos < self.tokens.len());
        ParserMark { pos: self.pos }
    }

    /// Rewind the position to the given mark.
    #[inline]
    pub fn rewind(&mut self, mark: ParserMark) {
        debug_assert!(mark.pos < self.tokens.len());
        self.pos = mark.pos;
    }

    /// Get a mark and return the span of the current position.
    #[inline]
    pub fn get_span_from(&self, mark: ParserMark) -> Span {
        debug_assert!(mark.pos < self.tokens.len());
        let start_token = self.tokens[mark.pos];
        let end_token = if self.pos > 0 {
            self.tokens[self.pos - 1]
        } else {
            self.tokens[0]
        };
        Span {
            source: self.source_id,
            start: start_token.span.start,
            end: end_token.span.end,
        }
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        &self.source.content[span.start as usize..span.end as usize]
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
        &self.source.content[token.span.start as usize..token.span.end as usize]
    }

    /// Get the current Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        self.tokens.get(self.pos)
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 1)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 2)
            .ok_or(ParseError::SyntaxError(self.eof_token.span))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat_next(&mut self) -> ParseResult<&TokenSpan> {
        if self.pos < self.tokens.len() {
            self.bump();
            let next = &self.tokens[self.pos - 1];
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(self.eof_token.span))
        }
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(self.pos < self.tokens.len(), "bump past end of tokens");
        self.pos += 1;
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::SyntaxError(next.span))
        }
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

    /// Peek a colon.
    #[inline]
    pub fn peek_colon(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Colon)
    }

    /// Eat a colon.
    #[inline]
    pub fn eat_colon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Colon)
    }

    /// Peek a semicolon.
    #[inline]
    pub fn peek_semicolon(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Semicolon)
    }

    /// Eat a semicolon.
    #[inline]
    pub fn eat_semicolon(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Semicolon)
    }

    /// Peek a comma.
    #[inline]
    pub fn peek_comma(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Comma)
    }

    /// Eat a comma.
    #[inline]
    pub fn eat_comma(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Comma)
    }

    /// Peek a newline.
    #[inline]
    pub fn peek_newline(&self) -> ParseResult<&TokenSpan> {
        self.peek_token(TokenType::Newline)
    }

    /// Eat a newline.
    #[inline]
    pub fn eat_newline(&mut self) -> ParseResult<&TokenSpan> {
        self.eat_token(TokenType::Newline)
    }

    /// Eat 0 or more newlines.
    #[inline]
    pub fn eat_newlines_maybe(&mut self) -> ParseResult<()> {
        while self.peek_newline().is_ok() {
            self.eat_newline()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
}
