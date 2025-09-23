use core::fmt;
use std::fmt::Debug;

use dyst_language_source::{Path, PathId, Source, SourceId, Span, StringId};
use dyst_language_token::{Token, TokenSpan, TokenType, is_semantic, tokenize_with_spans};

use crate::{Dumper, DumperOptions, EnclosingSpan, NodeTree, ParseError, ParseResult};
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
pub struct Parser<'ast> {
    /// The source we're parsing.
    pub source: &'ast Source,
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
    /// Whether the Parser has been finalized.
    pub(crate) is_finalized: bool,

    /// The Node AST tree.
    pub tree: NodeTree,
    /// The session.
    pub session: &'ast mut Session,
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
    /// Tokenizes immediately.
    pub fn from_source(source: &'a Source, session: &'a mut Session) -> Self {
        let (tokens, trivia_tokens) = tokenize_with_spans(source.id, &source.content, is_semantic);
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
            // source
            source,
            source_id: source.id,
            tokens,
            trivia_tokens,
            // state
            pos: 0,
            options: ParserOptions::default(),
            is_finalized: false,
            // result
            tree,
            session,
            eof_token,
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

    /// Finalize the parser.
    pub fn finalize(&mut self) {
        assert!(!self.is_finalized, "already finalized");
        self.is_finalized = true;
        self.attach_annotations();
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
            let diagnostic = e.to_diagnostic(self.source, &self.tokens);
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
        debug_assert!(!self.is_finalized, "bump after parser is finalized");
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

    /// Get the main (i.e. "biggest") node starting at a token.
    pub fn find_node_starting_at(&self, token: &TokenSpan) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(token.span.start, token.span.end.saturating_sub(1))
            .into_iter()
            .filter(|span| span.span.start == token.span.start)
            .collect::<Vec<_>>();
        enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
        enclosing_spans.into_iter().next()
    }

    /// Get the main (i.e. "biggest") node ending at a token.
    pub fn find_node_ending_at(&self, token: &TokenSpan) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(token.span.start, token.span.end.saturating_sub(1))
            .into_iter()
            .filter(|span| span.span.end == token.span.end)
            .collect::<Vec<_>>();
        enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
        enclosing_spans.into_iter().next()
    }

    /// Get the smallest, "highest" nodes enclosing a token.
    pub fn find_nodes_enclosing(&self, token: &TokenSpan) -> Vec<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(token.span.start, token.span.end.saturating_sub(1));
        if enclosing_spans.is_empty() {
            return Vec::new();
        }
        enclosing_spans.sort_by_key(|span| (span.length, -(span.idx as i64)));
        enclosing_spans
    }

    /// Get the smallest, "highest" node enclosing a token.
    pub fn find_node_enclosing(&self, token: &TokenSpan) -> Option<EnclosingSpan> {
        let enclosing_spans = self.find_nodes_enclosing(token);
        enclosing_spans.into_iter().next()
    }

    /// Check if two spans are on the same line.
    pub fn is_span_same_line(&self, left: Span, right: Span) -> bool {
        let left_line = self.source.get_position(left.start).map(|(line, _)| line);
        let right_line = self.source.get_position(right.start).map(|(line, _)| line);
        match (left_line, right_line) {
            (Some(lhs), Some(rhs)) => lhs == rhs,
            _ => false,
        }
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
