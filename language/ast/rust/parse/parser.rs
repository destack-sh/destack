use core::fmt;
use std::fmt::Debug;

use dyst_language_source::{Path, PathId, Source, SourceId, Span, StringId};
use dyst_language_token::{Token, TokenSpan, TokenType, is_semantic, tokenize_with_spans};

use crate::{Dumper, DumperOptions, NodeTree, ParseError, ParseResult};
use dyst_language_session::Session;

/// Configure Parser behavior.
/// Useful for enabling/disabling features in some AST subtrees.
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct ParserOptions {
    /// Whether we're parsing a static type (parameters or arguments).
    /// We disallow certain infix operations in static types to avoid ambiguity with <>.
    pub in_static_type: bool = false,
    /// Whether we're parsing an implicit union pattern.
    pub in_implicit_union: bool = false,
}

/// A parser for Dyst AST.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser<'a> {
    /// The source we're parsing.
    pub source: &'a Source,
    /// The source ID.
    pub source_id: SourceId,
    /// The semantic tokens parsed from the source.
    pub tokens: Vec<TokenSpan>,
    /// The trivia tokens parsed from the source.
    pub trivia_tokens: Vec<TokenSpan>,
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    pub eof_token: TokenSpan,

    /// The current position in the tokens.
    pos: usize,
    /// The parser options.
    pub(crate) options: ParserOptions,

    /// The Node tree.
    pub tree: NodeTree,
    /// The session.
    pub session: &'a mut Session,
    /// The errors encountered so far.
    pub errors: Vec<ParseError>,
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl<'a> Parser<'a> {
    /// Create a new parser from source.
    pub fn from_source(source: &'a Source, session: &'a mut Session) -> Self {
        let (tokens, trivia_tokens) = tokenize_with_spans(source.id, &source.content);
        let eof_token = *tokens.last().unwrap_or(&TokenSpan {
            span: Span {
                source: source.id,
                start: 0,
                end: 0,
            },
            token: Token::eof(),
        });
        let tree = NodeTree::new();
        Self {
            source,
            source_id: source.id,
            tokens,
            trivia_tokens,
            tree,
            session,
            pos: 0,
            eof_token,
            options: ParserOptions::default(),
            errors: Vec::new(),
        }
    }

    /// Create a new Dumper.
    pub fn dumper(&self, options: DumperOptions) -> Dumper<'_> {
        Dumper::new(
            &self.session.strings,
            &self.session.paths,
            &self.tree,
            options,
        )
    }

    /// Get the current position.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Execute a function with a new parser options.
    /// The previous options are restored after the function returns.
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

    /// Handle an error as a Diagnostic.
    /// Errors are deduplicated by leaf content to avoid squiggly red line noise.
    #[inline]
    pub(crate) fn handle_error(&mut self, e: &ParseError) {
        if !self.errors.iter().any(|d| d.eq_content(e)) {
            self.errors.push(e.clone());
            let diagnostic = e.into();
            self.session.handle_diagnostic(diagnostic);
        }
    }

    /// Intern a string.
    pub fn intern_string<S: AsRef<str>>(&mut self, string: S) -> StringId {
        self.session.intern_string(string)
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.session.get_string(string_id)
    }

    /// Intern a path.
    pub fn intern_path<T: AsRef<[StringId]>>(&mut self, path: T) -> PathId {
        self.session.intern(path)
    }

    /// Get an interned path.
    pub fn get_path(&self, path_id: PathId) -> &Path {
        self.session.get_path(path_id)
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark::new(self.pos)
    }

    /// Rewind the position to the given mark.
    #[inline]
    pub fn rewind(&mut self, mark: ParserMark) {
        self.pos = mark.pos;
    }

    /// Get a mark and return the span of the current position.
    #[inline]
    pub fn get_span_from(&self, mark: ParserMark) -> Span {
        // if we're beyond the end we just point to the EOF token
        if mark.pos >= self.tokens.len() {
            return self.eof_token.span;
        }

        // otherwise, get the span from the token
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
            .ok_or(ParseError::unexpected(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 1)
            .ok_or(ParseError::unexpected(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 2)
            .ok_or(ParseError::unexpected(self.eof_token.span))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParseResult<&TokenSpan> {
        if self.pos < self.tokens.len() {
            self.bump();
            let next = &self.tokens[self.pos - 1];
            Ok(next)
        } else {
            Err(ParseError::unexpected(self.eof_token.span))
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
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_token requires semantic token type"
        );
        let next = self.peek_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_next_token requires semantic token type"
        );
        let next = self.peek_next_next()?;
        if next.token.r#type == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "eat_token requires semantic token type"
        );
        let current = self.eat()?;
        if current.token.r#type == token_type {
            Ok(current)
        } else {
            Err(ParseError::unexpected(current.span))
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

    /// Attempt a function with recovery.
    pub fn with_recovery<T>(
        &mut self,
        start: ParserMark,
        func: impl FnOnce(&mut Self) -> ParseResult<T>,
        default: T,
        bail: TokenType,
    ) -> T {
        match func(self) {
            Ok(result) => result,
            Err(err) => {
                let _ = self.try_recover(start, bail, Some(err));
                default
            }
        }
    }

    /// Recover until the expected token.
    /// Everything from start to then is an error.
    pub fn try_recover(
        &mut self,
        start: ParserMark,
        recover: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        // let error = error.unwrap_or_else(|| ParseError::unexpected(self.get_span_from(start)));
        while let Ok(token) = self.peek() {
            // recover from here (but report error)
            if token.token.r#type == recover {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.handle_error(&error);
                return Ok(());
            } else {
                // keep going
                self.bump();
            }
        }
        // error if we didn't hit the expected token
        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.handle_error(&error);
        Err(error)
    }

    /// Eat the expected token.
    /// If we don't get the token, it's an error, but:
    ///  1) If we do hit the expected token later, we recover from there.
    ///  2) Otherwise, we try to recover forward until the bail token.
    pub fn try_eat_token(&mut self, expected: TokenType, bail: TokenType) -> ParseResult<()> {
        // we're good if it's the expected token
        if self.peek_token(expected).is_ok() {
            self.bump();
            return Ok(());
        }

        // try to recover
        let start = self.mark();
        while let Ok(token) = self.peek()
            && token.token.r#type != bail
        {
            // ok with error if we finally hit the expected token
            if token.token.r#type == expected {
                let error = ParseError::unexpected(self.get_span_from(start));
                self.bump();
                self.handle_error(&error);
                return Ok(());
            }
            // keep going
            else {
                self.bump();
            }
        }

        // error if we didn't hit the expected token, we're either at recovery or EOF
        let error = ParseError::unexpected(self.get_span_from(start));
        self.handle_error(&error);
        Err(error)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
}

impl ParserMark {
    /// Create a new ParserMark.
    #[inline]
    pub(crate) fn new(pos: usize) -> Self {
        Self { pos }
    }
}
