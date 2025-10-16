use core::fmt;
use std::fmt::Debug;

use crate::{TokenSpan, TokenType, is_semantic, tokenize_with_spans};
use dyst_source::{MultiSpan, Source, SourceId, Span, StringId, StringPool};

use crate::{
    Dumper, DumperOptions, EnclosingSpan, NodeSearch, NodeTree, NodeType, ParserError, ParserResult,
};
use dyst_session::Session;

/// Configure Parser behavior.
/// Useful for enabling/disabling features in some AST subtrees.
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct ParserOptions {
    // ------------------------------------------------------------
    // Contextual
    // (these usually stick inside nested contexts)
    // ------------------------------------------------------------

    /// Whether we're parsing inside a static argument (`<...>`).
    /// Disallows certain infix operations in static arguments to avoid ambiguity with <>.
    pub in_static: bool = false,
    /// Whether we're parsing inside a type.
    /// Type context eagerly evaluates some constructs to their type-ish variants.
    pub in_type: bool = false,

    // ------------------------------------------------------------
    // Structural
    // (these usually reset inside nested contexts)
    // ------------------------------------------------------------

    /// Whether we're parsing an expression before a type annotation (like the `x` in `x: int32`).
    /// Disallows binding patterns in these cases to avoid ambiguity with type annotations.
    pub in_before_type: bool = false,
    /// Whether we're parsing inside a match case.
    /// Disallows lambda functions to avoid ambiguity with match cases (`=>`).
    pub in_match_case: bool = false,
    /// Whether we're parsing a union pattern.
    /// Ignore elementwise infix operations in union patterns to avoid ambiguity with `|`
    pub in_union_pattern: bool = false,
    /// Whether we're in parenthesized expression (`(..)`, directly).
    /// These expressions might be tuple literals if followed by a comma.
    pub in_parenthesis: bool = false,
    /// Whether we're parsing an expression followed by a block (like in if, match, for, while).
    /// Disallows struct literals at the root level in these cases to avoid ambiguity with expr {}.
    pub in_before_block: bool = false,
    /// Whether we're in a tree literal.
    /// Disallows angle brackets and divides to avoid ambiguity with `</>``.
    pub in_tree_literal: bool = false,
    /// The left precedence preceding (i.e. before) the expression. 
    /// Determines expression operator lifting / grouping.
    pub left_precedence: Option<u16> = None,
}

impl ParserOptions {
    /// Adapt and reset options for a statement.
    pub(crate) fn in_statement(self) -> Self {
        Self::default()
    }

    /// Set `in_type=true`.
    pub(crate) fn in_type(self) -> Self {
        Self {
            in_type: true,
            ..self
        }
    }

    /// Set `in_union_pattern=true`.
    pub(crate) fn in_implicit_union(self) -> Self {
        Self {
            in_union_pattern: true,
            ..self
        }
    }

    /// Set `in_before_type=true`.
    pub(crate) fn in_before_type(self) -> Self {
        Self {
            in_before_type: true,
            ..self
        }
    }

    /// Set `in_before_block=true`.
    pub(crate) fn in_before_block(self) -> Self {
        Self {
            in_before_block: true,
            ..self
        }
    }

    /// Set `in_static=true` and `in_before_block=true`.
    pub(crate) fn static_in_before_block(self) -> Self {
        Self {
            in_static: true,
            in_before_block: true,
            ..self
        }
    }

    /// Set `in_tree_literal=true` and `in_parenthesis=false`.
    pub(crate) fn in_tree_literal(self) -> Self {
        Self {
            in_tree_literal: true,
            in_parenthesis: false,
            ..self
        }
    }

    /// Set `in_match_case=true`.
    pub(crate) fn in_match_case(self) -> Self {
        Self {
            in_match_case: true,
            ..self
        }
    }

    /// Set `left_precedence=precedence`.
    pub(crate) fn in_left_precedence(self, precedence: u16) -> Self {
        Self {
            left_precedence: Some(precedence),
            ..self
        }
    }

    /// Reset, set `in_static=true`.
    pub(crate) fn nested_in_static(self) -> Self {
        Self {
            in_static: true,
            ..Self::default()
        }
    }

    /// Reset, set `in_before_block=true`.
    pub(crate) fn nested_in_before_block(self) -> Self {
        Self {
            in_before_block: true,
            ..Self::default()
        }
    }

    /// Reset, set `in_type=true` and `in_before_block=true`.
    pub(crate) fn nested_type_in_before_block(self) -> Self {
        Self {
            in_type: true,
            in_before_block: true,
            ..Self::default()
        }
    }

    /// Reset, set `in_parenthesis=true`.
    pub(crate) fn nested_in_parenthesis(self) -> Self {
        Self {
            in_parenthesis: true,
            ..Self::default()
        }
    }

    /// Reset.
    pub(crate) fn nested(self) -> Self {
        Self::default()
    }
}

/// A parser for a single Dyst source's AST.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser<'ast> {
    /// The source we're parsing.
    pub source: &'ast Source,
    /// The source ID.
    pub source_id: SourceId,
    /// The current main tokens to consider.
    pub tokens: Vec<TokenSpan>,
    /// The side tokens not in the main tokens.
    pub side_tokens: Vec<TokenSpan>,
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
    /// The string pool.
    pub strings: StringPool,

    /// The session.
    pub session: &'ast mut Session,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParserError>,
}

impl Debug for Parser<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl<'a> Parser<'a> {
    /// Create a new parser from source and tokenize it.
    /// Also prepares the pre-annotations (like tags) in a pre-parse pass.
    pub fn prepare(source: &'a Source, session: &'a mut Session) -> Self {
        // tokenize
        let (all_tokens, eof_token) = tokenize_with_spans(source.id, &source.content);
        let (tokens, side_tokens) = all_tokens
            .iter()
            .partition(|token| is_semantic(token.token.ty));

        // make parser
        let mut parser = Self {
            // source
            source,
            source_id: source.id,
            // state
            tokens,
            side_tokens,
            pos: 0,
            options: ParserOptions::default(),
            is_finalized: false,
            // result
            tree: NodeTree::new(source.id),
            strings: StringPool::new(),
            session,
            eof_token,
            errors: Vec::new(),
        };

        // pre-parse side annotations
        parser.eat_side_annotations();
        // and then partition tokens
        let side_span = parser.get_side_span();
        let (tokens, side_tokens) = all_tokens
            .iter()
            .partition(|token| is_semantic(token.token.ty) && !side_span.contains(&token.span));
        parser.tokens = tokens;
        parser.side_tokens = side_tokens;

        // return the parser
        parser.reset();
        parser
    }

    /// Get the span of all side annotations.
    #[inline]
    pub fn get_side_span(&self) -> MultiSpan {
        let tag_spans = self.tree.get_spans_for(NodeType::Tag);
        let decorator_spans = self.tree.get_spans_for(NodeType::Decorator);
        MultiSpan::new(tag_spans.into_iter().chain(decorator_spans).collect())
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        self.pos = 0;
        self.options = ParserOptions::default();
        self.is_finalized = false;
        self.errors.clear();
    }

    /// Finalize the parser.
    pub fn finalize(&mut self) {
        assert!(!self.is_finalized, "already finalized");
        self.attach_annotations();
        self.is_finalized = true;
    }

    /// Create a new Dumper.
    pub fn dumper(&self, options: DumperOptions) -> Dumper<'_> {
        Dumper::new(&self.strings, &self.tree, options)
    }

    /// Get the current position in the tokens.
    #[inline]
    pub fn pos(&self) -> u32 {
        self.pos as u32
    }

    /// Execute a function with a new parser options.
    /// The previous options are restored after the function returns.
    #[inline]
    pub(crate) fn with_options<T>(
        &mut self,
        options: ParserOptions,
        func: impl FnOnce(&mut Self) -> ParserResult<T>,
    ) -> ParserResult<T> {
        let old_options = self.options;
        self.options = options;
        let result = func(self);
        self.options = old_options;
        result
    }

    /// Handle an error as a Diagnostic.
    /// Errors are deduplicated by leaf content to avoid squiggly red line noise.
    #[inline]
    pub(crate) fn handle_error(&mut self, e: &ParserError) {
        if !self.errors.iter().any(|d| d.eq_content(e)) {
            self.errors.push(e.clone());
            let diagnostic = e.to_diagnostic(self.source, &self.tokens);
            self.session.handle_diagnostic(diagnostic);
        }
    }

    /// Intern a string.
    pub fn intern_string<S: AsRef<str>>(&mut self, string: S) -> StringId {
        self.strings.intern(string)
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.strings.get(string_id)
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark::new(self.pos)
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn restore(&mut self, mark: ParserMark, idx: u32) {
        self.pos = mark.pos;
        self.tree.reset_to(idx);
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

    /// Get the previous Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        if self.pos > 0 {
            self.tokens.get(self.pos - 1)
        } else {
            None
        }
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&self) -> ParserResult<&TokenSpan> {
        self.tokens
            .get(self.pos)
            .ok_or(ParserError::unexpected(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&self) -> ParserResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 1)
            .ok_or(ParserError::unexpected(self.eof_token.span))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&self) -> ParserResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 2)
            .ok_or(ParserError::unexpected(self.eof_token.span))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParserResult<&TokenSpan> {
        if self.pos < self.tokens.len() {
            self.bump();
            let next = &self.tokens[self.pos - 1];
            Ok(next)
        } else {
            Err(ParserError::unexpected(self.eof_token.span))
        }
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(self.pos < self.tokens.len(), "bump past end of tokens");
        debug_assert!(!self.is_finalized, "bump after parser is finalized");
        self.pos += 1;
    }

    /// Bump the Token position by a given distance.
    #[inline]
    pub fn bump_by(&mut self, distance: u8) {
        debug_assert!(
            self.pos + (distance as usize) < self.tokens.len(),
            "bump past end of tokens"
        );
        debug_assert!(!self.is_finalized, "bump after parser is finalized");
        self.pos += distance as usize;
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&self, token_type: TokenType) -> ParserResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&self, token_type: TokenType) -> ParserResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_token requires semantic token type"
        );
        let next = self.peek_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParserResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_next_token requires semantic token type"
        );
        let next = self.peek_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next.span))
        }
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParserResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "eat_token requires semantic token type"
        );
        let current = self.eat()?;
        if current.token.ty == token_type {
            Ok(current)
        } else {
            Err(ParserError::unexpected(current.span))
        }
    }

    /// Attempt a function with recovery.
    pub fn with_recovery<T>(
        &mut self,
        start: ParserMark,
        func: impl FnOnce(&mut Self) -> ParserResult<T>,
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
        error: Option<ParserError>,
    ) -> ParserResult<()> {
        // let error = error.unwrap_or_else(|| ParseError::unexpected(self.get_span_from(start)));
        while let Ok(token) = self.peek() {
            // recover from here (but report error)
            if token.token.ty == recover {
                let error = ParserError::from_source_maybe(self.get_span_from(start), error);
                self.handle_error(&error);
                return Ok(());
            } else {
                // keep going
                self.bump();
            }
        }
        // error if we didn't hit the expected token
        let error = ParserError::from_source_maybe(self.get_span_from(start), error);
        self.handle_error(&error);
        Err(error)
    }

    /// Eat the expected token.
    /// If we don't get the token, it's an error, but:
    ///  1) If we do hit the expected token later, we recover from there.
    ///  2) Otherwise, we try to recover forward until the bail token.
    pub fn try_eat_token(&mut self, expected: TokenType, bail: TokenType) -> ParserResult<()> {
        // we're good if it's the expected token
        if self.peek_token(expected).is_ok() {
            self.bump();
            return Ok(());
        }

        // try to recover
        let start = self.mark();
        while let Ok(token) = self.peek()
            && token.token.ty != bail
        {
            // ok with error if we finally hit the expected token
            if token.token.ty == expected {
                let error = ParserError::unexpected(self.get_span_from(start));
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
        let error = ParserError::unexpected(self.get_span_from(start));
        self.handle_error(&error);
        Err(error)
    }

    /// Get the node starting at a token.
    pub fn find_node_starting_at(&self, span: &Span, search: NodeSearch) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(|s| s.span.start == span.start)
            .collect::<Vec<_>>();
        match search {
            NodeSearch::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearch::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearch::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Get the node ending at a token.
    pub fn find_node_ending_at(&self, span: &Span, search: NodeSearch) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(|s| s.span.end == span.end)
            .collect::<Vec<_>>();
        match search {
            NodeSearch::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearch::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearch::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Get the node enclosing a token.
    pub fn find_node_enclosing_at(
        &self,
        span: &Span,
        search: NodeSearch,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .spans
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(filter)
            .collect::<Vec<_>>();
        if enclosing_spans.is_empty() {
            return None;
        }
        match search {
            NodeSearch::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearch::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearch::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Check if two spans are on the same line.
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        let left_line = self.source.get_position(left.start).map(|(line, _)| line);
        let right_line = self.source.get_position(right.end).map(|(line, _)| line);
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
