use core::fmt;
use std::fmt::Debug;
use std::sync::Arc;

use crate::{Lexer, is_semantic};
use destack_ast::{BlockFormat, Expression, LocalNodeId, NodeTree, NodeType, TokenSpan, TokenType};
use destack_base::LocalStringPool;
use destack_source::{
    DiagnosticCollector, EnclosingSpan, File, FileId, LanguageType, MultiSpan, NodeSearchMode, Span,
};

use crate::{ParseError, ParseResult};

/// Configure Parser behavior.
/// Useful for enabling/disabling features in some AST subtrees.
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct ParserOptions {
    /// Whether we're parsing inside a static argument (`<...>`).
    /// Disallows certain infix operations in static arguments to avoid ambiguity with <>.
    pub in_static: bool = false,
    /// Whether we're parsing inside a comptime expression.
    /// Enables limited type-only parsing when unambiguous.
    pub in_comptime: bool = false,
    /// Whether we're parsing inside a type.
    /// Type context eagerly evaluates some constructs to their type-ish variants.
    pub in_type: bool = false,
    /// Whether we're inside a super type (clause)
    /// Binary type operators are prohibited in super type clauses.
    pub in_super_type: bool = false,
    /// Whether we're parsing inside a variant.
    /// Certain properties/constructs are only allowed inside variants.
    pub in_variant: bool = false,
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
    /// Whether we're parsing at the start of a "statement".
    /// Disallows anonymous struct literals.
    pub in_statement_position: bool = false,
    /// Whether we're parsing an expression followed by a block (like in if, match, for, while).
    /// Disallows all struct literals at the root level in these cases to avoid ambiguity with `expr {}`.
    pub in_before_block: bool = false,
    /// Whether we're in a tree literal.
    /// Disallows angle brackets and divides to avoid ambiguity with `</>``.
    pub in_tree_literal: bool = false,
    /// Whether we're parsing a ternary if expression.
    /// Disallows some shorthand syntax like lambdas that looks like a ternary part.
    pub in_ternary_condition: bool = false,
    /// Whether we're parsing the right side of a type conditional.
    /// Stops the parse at `?` so the outer conditional can consume it.
    pub in_type_conditional_right: bool = false,
    /// Whether we're parsing a for each expression.
    /// Disallows container operators.
    pub in_for_each: bool = false,
    /// Whether we're parsing a new receiver.
    /// Disallows call-like expressions to disambiguate dynamic arguments.
    pub in_new_receiver: bool = false,
    /// Whether we're parsing inside a generator function.
    /// Makes `yield` a keyword instead of an identifier.
    pub in_generator: bool = false,
    /// The left precedence preceding (i.e. before) the expression.
    /// Determines expression operator lifting / grouping.
    pub left_precedence: Option<u16> = None,
}

#[allow(unused)]
impl ParserOptions {
    /// Set `in_static=true`.
    #[inline]
    pub(crate) fn in_static(self) -> Self {
        Self {
            in_static: true,
            ..self
        }
    }

    /// Set `in_comptime=true`.
    #[inline]
    pub(crate) fn in_comptime(self) -> Self {
        Self {
            in_comptime: true,
            ..self
        }
    }

    /// Set `in_type=true`.
    #[inline]
    pub(crate) fn in_type(self) -> Self {
        Self {
            in_type: true,
            ..self
        }
    }

    /// Set `in_super_type=true`.
    #[inline]
    pub(crate) fn in_super_type(self) -> Self {
        Self {
            in_type: true,
            in_super_type: true,
            ..self
        }
    }

    /// Set `in_variant=true`.
    #[inline]
    pub(crate) fn in_variant(self) -> Self {
        Self {
            in_variant: true,
            ..self
        }
    }

    /// Set `in_before_type=true`.
    #[inline]
    pub(crate) fn in_before_type(self) -> Self {
        Self {
            in_before_type: true,
            ..self
        }
    }

    /// Set `in_match_case=true`.
    #[inline]
    pub(crate) fn in_match_case(self) -> Self {
        Self {
            in_match_case: true,
            ..self
        }
    }

    /// Set `in_union_pattern=true`.
    #[inline]
    pub(crate) fn in_union_pattern(self) -> Self {
        Self {
            in_union_pattern: true,
            ..self
        }
    }

    /// Set `in_parenthesis=true`.
    #[inline]
    pub(crate) fn in_parenthesis(self) -> Self {
        Self {
            in_parenthesis: true,
            ..self
        }
    }

    /// Set `in_statement_position=true`.
    #[inline]
    pub(crate) fn in_statement_position(self) -> Self {
        Self {
            in_statement_position: true,
            ..self
        }
    }

    /// Set `in_before_block=true`.
    #[inline]
    pub(crate) fn in_before_block(self) -> Self {
        Self {
            in_before_block: true,
            ..self
        }
    }

    /// Set `in_tree_literal=true`.
    #[inline]
    pub(crate) fn in_tree_literal(self) -> Self {
        Self {
            in_tree_literal: true,
            ..self
        }
    }

    /// Set `in_tree_literal=false`.
    #[inline]
    pub(crate) fn not_in_tree_literal(self) -> Self {
        Self {
            in_tree_literal: false,
            ..self
        }
    }

    /// Set `in_ternary_condition=true`.
    #[inline]
    pub(crate) fn in_ternary_condition(self) -> Self {
        Self {
            in_ternary_condition: true,
            ..self
        }
    }

    /// Set `in_type_conditional_right=true`.
    #[inline]
    pub(crate) fn in_type_conditional_right(self) -> Self {
        Self {
            in_type_conditional_right: true,
            ..self
        }
    }

    /// Set `in_for_each=true`.
    #[inline]
    pub(crate) fn in_for_each(self) -> Self {
        Self {
            in_for_each: true,
            ..self
        }
    }

    /// Set `in_new_receiver=true`.
    #[inline]
    pub(crate) fn in_new_receiver(self) -> Self {
        Self {
            in_new_receiver: true,
            ..self
        }
    }

    /// Set `in_generator=true`.
    #[inline]
    pub(crate) fn in_generator(self) -> Self {
        Self {
            in_generator: true,
            ..self
        }
    }

    /// Set `left_precedence=precedence`.
    #[inline]
    pub(crate) fn in_left_precedence(self, precedence: u16) -> Self {
        Self {
            left_precedence: Some(precedence),
            ..self
        }
    }

    /// Clear `left_precedence` to allow all operators.
    #[inline]
    pub(crate) fn not_in_left_precedence(self) -> Self {
        Self {
            left_precedence: None,
            ..self
        }
    }

    /// Not previous position.
    #[inline]
    pub(crate) fn not_in_position(self) -> Self {
        Self {
            in_parenthesis: false,
            in_statement_position: false,
            in_type_conditional_right: false,
            ..self
        }
    }

    /// Reset position-related options but preserve context options like `in_generator`.
    pub(crate) fn nested(self) -> Self {
        Self {
            in_generator: self.in_generator,
            in_comptime: self.in_comptime,
            ..Self::default()
        }
    }
}

/// A parser for a single Destack source's AST.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser {
    /// The source we're parsing.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The current main tokens to consider.
    pub tokens: Vec<TokenSpan>,
    /// The side tokens not in the main tokens.
    pub side_tokens: Vec<TokenSpan>,
    /// The EOF token (the actual last token or a fake placeholder one if empty).
    pub eof_token: TokenSpan,

    /// The current position in the tokens.
    pos: usize,
    /// Whether the parser is finished.
    is_finished: bool,
    /// The parser options.
    pub(crate) options: ParserOptions,

    /// The Node AST tree.
    pub tree: NodeTree,
    /// The string pool (lockless for single-threaded parsing).
    pub strings: LocalStringPool,

    /// The language type for parsing behavior.
    pub language: LanguageType,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParseError>,
    /// Scratch storage for annotation tokens to avoid repeated allocations.
    pub(crate) annotation_tokens: Vec<TokenSpan>,
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Create a new parser from a text File and tokenize it.
    /// Also prepares the pre-annotations (like tags) in a pre-parse pass.
    #[tracing::instrument(name = "parser.lex", level = "trace", skip_all, fields(file_id = ?file.id))]
    pub fn lex_file(file: Arc<File>, language: LanguageType) -> Self {
        // tokenize directly into semantic and side token vecs (no partition needed)
        let (tokens, side_tokens, eof_token) = Lexer::lex(file.id, file.text(), language);

        // make parser with estimated capacity
        // roughly 1 AST node per 3 tokens on average
        let estimated_nodes = tokens.len() / 3;
        let file_id = file.id;
        let token_capacity = tokens.len() + side_tokens.len();
        let mut parser = Self {
            file,
            file_id,
            tokens,
            side_tokens,
            pos: 0,
            is_finished: false,
            options: ParserOptions::default(),
            language,
            tree: NodeTree::with_capacity(estimated_nodes),
            strings: LocalStringPool::new(),
            diagnostics: DiagnosticCollector::new(),
            eof_token,
            errors: Vec::new(),
            annotation_tokens: Vec::with_capacity(token_capacity),
        };

        // pre-parse side annotations (decorators)
        parser.eat_side_annotations();

        // move decorator tokens from main tokens to side tokens
        let side_span = parser.compute_side_span();
        if !side_span.spans.is_empty() {
            let (new_tokens, decorator_tokens): (Vec<_>, Vec<_>) = parser
                .tokens
                .drain(..)
                .partition(|token| !side_span.contains(&token.span));
            parser.tokens = new_tokens;
            parser.side_tokens.extend(decorator_tokens);
            parser
                .side_tokens
                .sort_by_key(|token| (token.span.start, token.span.end));
        }

        // return the parser
        parser.reset();
        parser
    }

    /// Get the span of all side annotations.
    #[inline]
    pub fn compute_side_span(&self) -> MultiSpan {
        Self::compute_side_span_from_tree(&self.tree)
    }

    /// Get the span of all side annotations from a tree.
    /// Uses Decorator spans since these exist before Annotation nodes are created.
    #[inline]
    pub fn compute_side_span_from_tree(tree: &NodeTree) -> MultiSpan {
        let decorator_spans = tree.get_spans_for(NodeType::Decorator);
        MultiSpan::new(decorator_spans)
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.pos = 0;
        self.options = ParserOptions::default();
        self.errors.clear();
    }

    /// Parse everything as an implicit namespace (without creating the namespace).
    #[tracing::instrument(name = "parser.parse", level = "trace", skip_all, fields(file_id = ?self.file_id))]
    pub fn parse(&mut self) -> Vec<LocalNodeId<Expression>> {
        // parse main expressions without finalization
        let expressions = self.parse_without_finish();

        // finalize annotations and indexes
        self.finish();

        expressions
    }

    /// Parse everything as an implicit namespace without attaching annotations or indexes.
    /// Call `finish` to attach annotations and build the position index.
    pub fn parse_without_finish(&mut self) -> Vec<LocalNodeId<Expression>> {
        // parse the root block body with recovery
        let mut expressions = self.with_recovery(
            self.mark(),
            |parser| parser.eat_block_body(BlockFormat::Implicit),
            Vec::new(),
            TokenType::End,
        );

        // insert a stub expression if there are no expressions but there are annotations
        // (this ensures annotations have something to attach to, e.g. in comment-only files)
        if expressions.is_empty() && self.has_annotation_tokens() {
            let stub = self.tree.insert(Expression::Stub, self.file_span());
            expressions.push(stub);
        }

        expressions
    }

    /// Check if there are any annotation tokens (comments, docs) in the side tokens.
    fn has_annotation_tokens(&self) -> bool {
        self.side_tokens.iter().any(|token| {
            matches!(
                token.token.ty,
                TokenType::LineComment
                    | TokenType::DocLineComment
                    | TokenType::BlockComment
                    | TokenType::DocBlockComment
            )
        })
    }

    /// Get a span covering the entire file.
    fn file_span(&self) -> Span {
        Span::new(self.file_id, 0, self.eof_token.span.end)
    }

    /// Finish parsing. You don't need to call this manually if using Parser::parse().
    pub fn finish(&mut self) {
        if !self.is_finished {
            // build position index BEFORE annotation attachment for O(log n) lookups
            self.tree.build_position_index();
            self.attach_annotations();
            self.is_finished = true;
        }
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
    pub(crate) fn error(&mut self, e: &ParseError) {
        if !self.errors.iter().any(|d| d.eq_content(e)) {
            self.errors.push(e.clone());
            let diagnostic = e.to_diagnostic(self.file.as_ref(), &self.tokens);
            self.diagnostics.insert(diagnostic);
        }
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark::new(self.pos)
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn rewind(&mut self, mark: ParserMark) {
        self.pos = mark.pos;
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
            file: self.file_id,
            start: start_token.span.start,
            end: end_token.span.end,
        }
    }

    /// Get the span between two marks.
    #[inline]
    pub fn get_span_between(&self, start: ParserMark, end: ParserMark) -> Span {
        let start_token = self.tokens[start.pos];
        let end_token = self.tokens[end.pos];
        Span::new(self.file_id, start_token.span.start, end_token.span.end)
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &str {
        self.file.get_span_str(span).unwrap_or_default()
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &str {
        self.file.get_span_str(token.span).unwrap_or_default()
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

    /// Get the previous Token type.
    #[inline]
    pub fn prev_token_type(&self) -> TokenType {
        self.prev()
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
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

    /// Peek the next next next Token or error.
    #[inline]
    pub fn peek_next_next_next(&self) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + 3)
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
        debug_assert!(!self.is_finished, "parser is already finished");
        debug_assert!(self.pos < self.tokens.len(), "bump past end of tokens");
        self.pos += 1;
    }

    /// Bump the Token position by a given distance.
    #[inline]
    pub fn bump_by(&mut self, distance: u8) {
        debug_assert!(!self.is_finished, "parser is already finished");
        debug_assert!(
            self.pos + (distance as usize) < self.tokens.len(),
            "bump past end of tokens"
        );
        self.pos += distance as usize;
    }

    /// Peek a token at a position.
    #[inline]
    pub fn peek_token_ahead(&self, delta: u32, token_type: TokenType) -> ParseResult<&TokenSpan> {
        self.tokens
            .get(self.pos + (delta as usize))
            .filter(|token| token.token.ty == token_type)
            .ok_or(ParseError::unexpected(self.eof_token.span))
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_token_in(&self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek()?;
        if token_types.contains(&next.token.ty) {
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
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_next_token_in(&self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next token in a list of token types.
    #[inline]
    pub fn peek_next_next_token_in(&self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_next_token(&self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token in a list of token types.
    #[inline]
    pub fn peek_next_next_next_token_in(
        &self,
        token_types: &[TokenType],
    ) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next_next()?;
        if token_types.contains(&next.token.ty) {
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
        if current.token.ty == token_type {
            Ok(current)
        } else {
            Err(ParseError::unexpected(current.span))
        }
    }

    /// Eat a token maybe.
    pub fn eat_token_maybe(&mut self, token_type: TokenType) -> ParseResult<bool> {
        if self.peek_token(token_type).is_ok() {
            self.bump();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Eat a token in a list of tokens.
    #[inline]
    pub fn eat_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<TokenType> {
        let current = self.eat()?;
        if token_types.contains(&current.token.ty) {
            Ok(current.token.ty)
        } else {
            Err(ParseError::unexpected(current.span))
        }
    }

    /// Eat a token in a list of tokens maybe.
    #[inline]
    pub fn eat_token_in_maybe(
        &mut self,
        token_types: &[TokenType],
    ) -> ParseResult<Option<TokenType>> {
        let token = *self.peek()?;
        if token_types.contains(&token.token.ty) {
            self.bump();
            Ok(Some(token.token.ty))
        } else {
            Ok(None)
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
            if token.token.ty == recover {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            } else {
                // keep going
                self.bump();
            }
        }
        // error if we didn't hit the expected token
        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);
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
            && token.token.ty != bail
        {
            // ok with error if we finally hit the expected token
            if token.token.ty == expected {
                let error = ParseError::unexpected(self.get_span_from(start));
                self.bump();
                self.error(&error);
                return Ok(());
            }
            // keep going
            else {
                self.bump();
            }
        }

        // error if we didn't hit the expected token, we're either at recovery or EOF
        let error = ParseError::unexpected(self.get_span_from(start));
        self.error(&error);
        Err(error)
    }

    /// Get the node starting at a token.
    pub fn find_node_starting_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .source_map
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(|s| s.span.start == span.start)
            .collect::<Vec<_>>();
        match search {
            NodeSearchMode::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearchMode::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearchMode::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Get the node ending at a token.
    pub fn find_node_ending_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .source_map
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(|s| s.span.end == span.end)
            .collect::<Vec<_>>();
        match search {
            NodeSearchMode::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearchMode::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearchMode::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Get the node enclosing a token.
    pub fn find_node_enclosing_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        let mut enclosing_spans = self
            .tree
            .source_map
            .get_enclosing_spans(span.start, span.end.saturating_sub(1))
            .into_iter()
            .filter(filter)
            .collect::<Vec<_>>();
        if enclosing_spans.is_empty() {
            return None;
        }
        match search {
            NodeSearchMode::BiggestOutermost => {
                enclosing_spans.sort_by_key(|span| (-(span.length as i64), -(span.idx as i64)));
            }
            NodeSearchMode::SmallestOutermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, -(span.idx as i64)));
            }
            NodeSearchMode::SmallestInnermost => {
                enclosing_spans.sort_by_key(|span| (span.length as i64, (span.idx as i64)));
            }
        }
        enclosing_spans.into_iter().next()
    }

    /// Check if two spans are on the same line.
    #[inline]
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        self.file.is_same_line(left.start, right.end)
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
