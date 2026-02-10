use core::fmt;
use std::fmt::Debug;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use crate::{TokenStream, TokenStreamMark, is_semantic};
use destack_ast::{
    BlockFormat, Decorator, Expression, Keyword, LocalNodeId, NodeTree, NodeType, StringId, Token,
    TokenSpan, TokenType,
};
use destack_base::LocalStringPool;
use destack_source::{
    DiagnosticCollector, EnclosingSpan, File, FileId, LanguageType, MultiSpan, NodeSearchMode, Span,
};

use crate::parse::timing::{ParserTimingScope, ParserTimings, tags};
use crate::{ParseError, ParseResult};

/// Cached string ids for type literal identifiers.
pub(crate) struct TypeLiteralIdentifiers {
    pub(crate) undefined: StringId,
    pub(crate) unknown: StringId,
    pub(crate) object: StringId,
    pub(crate) null_: StringId,
    pub(crate) any: StringId,
    pub(crate) never: StringId,
    pub(crate) boolean: StringId,
    pub(crate) void: StringId,
    pub(crate) character: StringId,
    pub(crate) string: StringId,
    pub(crate) bigint: StringId,
    pub(crate) number: StringId,
    pub(crate) int: StringId,
    pub(crate) isize: StringId,
    pub(crate) uint: StringId,
    pub(crate) usize: StringId,
    pub(crate) float: StringId,
    pub(crate) symbol: StringId,
    pub(crate) unique: StringId,
}

impl TypeLiteralIdentifiers {
    /// Create cached ids for the current string pool.
    fn new(strings: &mut LocalStringPool) -> Self {
        Self {
            undefined: strings.intern("undefined"),
            unknown: strings.intern("unknown"),
            object: strings.intern("object"),
            null_: strings.intern("null"),
            any: strings.intern("any"),
            never: strings.intern("never"),
            boolean: strings.intern("boolean"),
            void: strings.intern("void"),
            character: strings.intern("character"),
            string: strings.intern("string"),
            bigint: strings.intern("bigint"),
            number: strings.intern("number"),
            int: strings.intern("int"),
            isize: strings.intern("isize"),
            uint: strings.intern("uint"),
            usize: strings.intern("usize"),
            float: strings.intern("float"),
            symbol: strings.intern("symbol"),
            unique: strings.intern("unique"),
        }
    }
}

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
    /// Whether we're parsing inside an ambient declaration context.
    pub in_declare_context: bool = false,
    /// Whether we're in parenthesized expression (`(..)`, directly).
    /// These expressions might be tuple literals if followed by a comma.
    pub in_parenthesis: bool = false,
    /// Whether we're parsing at the start of a "statement".
    /// Disallows object literals.
    pub in_statement_position: bool = false,
    /// Whether we're parsing an expression followed by a block (like in if, match, for, while).
    /// Disallows object and typed struct literals at the root level to avoid ambiguity with `expr {}`.
    pub in_before_block: bool = false,
    /// Whether we're in a tree literal.
    /// Disallows angle brackets and divides to avoid ambiguity with `</>``.
    pub in_tree_literal: bool = false,
    /// Whether we're parsing a decorator expression.
    /// Disables declaration and control flow keyword parsing to keep decorators as expressions.
    pub in_decorator: bool = false,
    /// Whether we're parsing a ternary if expression.
    /// Disallows some shorthand syntax like lambdas that looks like a ternary part.
    pub in_ternary_condition: bool = false,
    /// Whether we're parsing the right side of a type conditional.
    /// Stops the parse at `?` so the outer conditional can consume it.
    pub in_type_conditional_right: bool = false,
    /// Whether we're parsing a return type before an arrow body.
    /// Stops lambdas from consuming the outer `=>`.
    pub in_arrow_return_type: bool = false,
    /// Whether we're parsing a mapped type constraint.
    /// Disables `as` casts so the remap clause can be parsed separately.
    pub in_type_mapped_constraint: bool = false,
    /// Whether we're parsing a for each expression.
    /// Disallows container operators.
    pub in_for_each: bool = false,
    /// Whether we're parsing a new receiver.
    /// Disallows call-like expressions to disambiguate dynamic arguments.
    pub in_new_receiver: bool = false,
    /// Whether we're parsing a typeof type query operand.
    /// Keeps contextual keywords available as identifier paths.
    pub in_typeof_query: bool = false,
    /// Whether we're parsing inside a generator function.
    /// Makes `yield` a keyword instead of an identifier.
    pub in_generator: bool = false,
    /// Whether `yield` expressions are forbidden in this context.
    /// Used to tag contexts where `yield` should be rejected during analysis.
    pub forbid_yield: bool = false,
    /// Whether `await` expressions are forbidden in this context.
    pub forbid_await: bool = false,
    /// Whether sequence expressions (comma operator) are allowed.
    pub allow_sequence_expression: bool = true,
    /// Whether private hash keys (`#name`) are allowed in key position.
    pub allow_private_hash_key: bool = false,
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool = false,
    /// The left precedence preceding (i.e. before) the expression.
    /// Determines expression operator lifting / grouping.
    pub left_precedence: Option<u16> = None,
}

/// Parser settings that can be configured externally.
/// NOTE #Cleanup: ParserSettings living separately from Parser and ParserOptions feels awkward.
#[derive(Debug, Copy, Clone, Default)]
pub struct ParserSettings {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
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

    /// Set `in_type=false`.
    #[inline]
    pub(crate) fn not_in_type(self) -> Self {
        Self {
            in_type: false,
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

    /// Set `in_generator` to the given value.
    #[inline]
    pub(crate) fn with_generator(self, in_generator: bool) -> Self {
        Self {
            in_generator,
            ..self
        }
    }

    /// Set `forbid_yield` to the given value.
    #[inline]
    pub(crate) fn with_forbid_yield(self, forbid_yield: bool) -> Self {
        Self {
            forbid_yield,
            ..self
        }
    }

    /// Set `forbid_await=true`.
    #[inline]
    pub(crate) fn forbid_await(self) -> Self {
        Self {
            forbid_await: true,
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

    /// Set `in_declare_context=true`.
    #[inline]
    pub(crate) fn in_declare_context(self) -> Self {
        Self {
            in_declare_context: true,
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

    /// Set `in_before_block=false`.
    #[inline]
    pub(crate) fn not_in_before_block(self) -> Self {
        Self {
            in_before_block: false,
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

    /// Set `in_ternary_condition=false`.
    #[inline]
    pub(crate) fn not_in_ternary_condition(self) -> Self {
        Self {
            in_ternary_condition: false,
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

    /// Set `in_arrow_return_type=true`.
    #[inline]
    pub(crate) fn in_arrow_return_type(self) -> Self {
        Self {
            in_arrow_return_type: true,
            ..self
        }
    }

    /// Set `in_type_mapped_constraint=true`.
    #[inline]
    pub(crate) fn in_type_mapped_constraint(self) -> Self {
        Self {
            in_type_mapped_constraint: true,
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

    /// Set `in_new_receiver=false`.
    #[inline]
    pub(crate) fn not_in_new_receiver(self) -> Self {
        Self {
            in_new_receiver: false,
            ..self
        }
    }

    /// Set `in_typeof_query=true`.
    #[inline]
    pub(crate) fn in_typeof_query(self) -> Self {
        Self {
            in_typeof_query: true,
            ..self
        }
    }

    /// Set `allow_private_hash_key=true`.
    #[inline]
    pub(crate) fn allow_private_hash_key(self) -> Self {
        Self {
            allow_private_hash_key: true,
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

    /// Set `in_decorator=true`.
    #[inline]
    pub(crate) fn in_decorator(self) -> Self {
        Self {
            in_decorator: true,
            ..self
        }
    }

    /// Set `in_decorator=false`.
    #[inline]
    pub(crate) fn not_in_decorator(self) -> Self {
        Self {
            in_decorator: false,
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

    /// Disallow sequence expressions (comma operator).
    #[inline]
    pub(crate) fn not_in_sequence_expression(self) -> Self {
        Self {
            allow_sequence_expression: false,
            ..self
        }
    }

    /// Disallow arrow return type shielding for nested expressions.
    #[inline]
    pub(crate) fn not_in_arrow_return_type(self) -> Self {
        Self {
            in_arrow_return_type: false,
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
            forbid_yield: self.forbid_yield,
            forbid_await: self.forbid_await,
            allow_sequence_expression: self.allow_sequence_expression,
            in_decorator: self.in_decorator,
            disallow_ambiguous_tree_literal: self.disallow_ambiguous_tree_literal,
            in_declare_context: self.in_declare_context,
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
    /// The token stream driving the parser.
    pub(crate) token_stream: TokenStream,

    /// The current position in the tokens.
    pos: usize,
    /// Pending token from splitting a compound token (e.g., second `<` from `<<`).
    split_token: Option<TokenSpan>,
    /// Whether the split token has been consumed (but data kept for eat() to return).
    split_token_consumed: bool,
    /// Whether the parser is finished.
    is_finished: bool,
    /// Whether the position index is built.
    pub(crate) positions_built: bool,
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
    /// Scratch storage for annotation line indices to avoid repeated allocations.
    pub(crate) annotation_line_indices: Vec<u32>,
    /// Optional parser timing collector.
    pub(crate) timings: Option<Rc<ParserTimings>>,
    /// Cached keyword lookup for identifier tokens.
    pub(crate) token_keywords: Vec<Option<Keyword>>,
    /// Cached identifier lookup for identifier tokens.
    pub(crate) token_identifiers: Vec<Option<StringId>>,
    /// Cached string id for `global`.
    pub(crate) global_identifier: Option<StringId>,
    /// Cached string id for `module`.
    pub(crate) module_identifier: Option<StringId>,
    /// Cached string id for `_`.
    pub(crate) underscore_identifier: StringId,
    /// Cached identifiers used by type literal parsing.
    pub(crate) type_literal_identifiers: TypeLiteralIdentifiers,
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
        // initialize token stream for lazy lexing
        let token_stream = TokenStream::new(file.clone(), language);

        // make parser with estimated capacity
        // roughly 1 AST node per 3 tokens on average
        let estimated_tokens = file.text().len() / 6;
        let estimated_nodes = estimated_tokens / 3;
        let file_id = file.id;
        let token_capacity = estimated_tokens * 2;
        let mut strings = LocalStringPool::new();
        let type_literal_identifiers = TypeLiteralIdentifiers::new(&mut strings);
        let global_identifier = Some(strings.intern("global"));
        let module_identifier = Some(strings.intern("module"));
        let underscore_identifier = strings.intern("_");
        let mut parser = Self {
            file,
            file_id,
            token_stream,
            pos: 0,
            split_token: None,
            split_token_consumed: false,
            is_finished: false,
            positions_built: false,
            options: ParserOptions::default(),
            language,
            tree: NodeTree::with_capacity(estimated_nodes),
            strings,
            diagnostics: DiagnosticCollector::new(),
            errors: Vec::new(),
            annotation_tokens: Vec::with_capacity(token_capacity),
            annotation_line_indices: Vec::with_capacity(token_capacity),
            timings: timings_enabled_from_env().then(|| Rc::new(ParserTimings::default())),
            token_keywords: Vec::new(),
            token_identifiers: Vec::new(),
            global_identifier,
            module_identifier,
            underscore_identifier,
            type_literal_identifiers,
        };

        // reset parser state to start
        parser.reset();
        parser
    }

    /// Lex a file and apply parser settings.
    #[tracing::instrument(
        name = "parser.lex",
        level = "trace",
        skip_all,
        fields(file_id = ?file.id)
    )]
    pub fn lex_file_with_settings(
        file: Arc<File>,
        language: LanguageType,
        settings: ParserSettings,
    ) -> Self {
        let mut parser = Self::lex_file(file, language);
        parser.apply_settings(settings);
        parser
    }

    /// Apply externally provided parser settings.
    #[inline]
    pub fn apply_settings(&mut self, settings: ParserSettings) {
        self.options.disallow_ambiguous_tree_literal = settings.disallow_ambiguous_tree_literal;
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
        self.split_token = None;
        self.options = ParserOptions {
            disallow_ambiguous_tree_literal: self.language.supports_jsx()
                && self.language.is_typescript(),
            ..ParserOptions::default()
        };
        self.errors.clear();
        self.positions_built = false;
    }

    /// Start a parser timing scope.
    pub(crate) fn timing_scope(
        &self,
        tag: crate::parse::timing::ParserTimingTag,
    ) -> ParserTimingScope {
        ParserTimingScope::new(self.timings.clone(), tag)
    }

    /// Snapshot timing entries recorded by the parser.
    pub fn timing_snapshot(&self) -> Option<Vec<crate::parse::timing::ParserTimingEntry>> {
        self.timings.as_ref().map(|timings| timings.snapshot())
    }

    /// Return the current semantic tokens.
    #[inline]
    pub(crate) fn tokens(&self) -> &[TokenSpan] {
        self.token_stream.tokens()
    }

    /// Return the current side tokens.
    #[inline]
    pub(crate) fn side_tokens(&self) -> &[TokenSpan] {
        self.token_stream.side_tokens()
    }

    /// Return owned token buffers after lexing to EOF.
    pub fn take_tokens(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        self.token_stream.take_tokens()
    }

    /// Lex tokens to EOF without changing parser position.
    #[inline]
    pub(crate) fn lex_to_end(&mut self) {
        self.token_stream.lex_to_end();
    }

    /// Return the EOF span without forcing a full lex.
    #[inline]
    pub(crate) fn eof_span(&self) -> Span {
        Span::new(self.file_id, self.file.len, self.file.len)
    }

    /// Ensure a token exists at the given index and return it.
    #[inline]
    pub(crate) fn token_at(&mut self, index: usize) -> Option<TokenSpan> {
        self.token_stream.token(index)
    }

    /// Ensure a token exists at the given index and return a reference.
    #[inline]
    pub(crate) fn token_ref_at(&mut self, index: usize) -> Option<&TokenSpan> {
        self.token_stream.ensure_token(index);
        self.tokens().get(index)
    }

    /// Look up the token type at a given index.
    #[inline]
    pub(crate) fn token_type_at(&mut self, index: usize) -> TokenType {
        self.token_stream.ensure_token(index);
        self.tokens()
            .get(index)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Return true when two tokens touch in the source with no whitespace.
    #[inline]
    pub(crate) fn tokens_are_adjacent(&mut self, left_index: usize, right_index: usize) -> bool {
        let Some(left) = self.token_at(left_index) else {
            return false;
        };
        let Some(right) = self.token_at(right_index) else {
            return false;
        };

        left.span.end == right.span.start
    }

    /// Check that two tokens are adjacent (no whitespace between them).
    #[inline]
    pub(crate) fn check_tokens_are_adjacent(
        &mut self,
        left_index: usize,
        right_index: usize,
    ) -> ParseResult<()> {
        if self.tokens_are_adjacent(left_index, right_index) {
            return Ok(());
        }

        let span = self
            .token_ref_at(right_index)
            .map(|token| token.span)
            .unwrap_or(self.eof_span());
        Err(ParseError::unexpected(span))
    }

    /// Truncate token caches to match the current token count.
    fn truncate_token_caches(&mut self) {
        let len = self.tokens().len();
        self.token_keywords.truncate(len);
        self.token_identifiers.truncate(len);
    }

    /// Return true when a split token is active.
    #[inline]
    pub(crate) fn has_active_split(&self) -> bool {
        self.split_token.is_some() && !self.split_token_consumed
    }

    /// Get the current token index.
    #[inline]
    pub(crate) fn pos_index(&self) -> usize {
        self.pos
    }

    /// Look up a keyword at a token index.
    #[inline]
    pub(crate) fn keyword_for_index(&mut self, index: usize) -> Option<Keyword> {
        self.token_stream.ensure_token(index);

        if self.token_keywords.len() <= index {
            self.token_keywords.resize(index + 1, None);
        }

        if self.token_keywords[index].is_some() {
            return self.token_keywords[index];
        }

        let token = self.tokens().get(index)?;
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        let keyword = Keyword::from_str(self.get_span_str(token.span)).ok();
        self.token_keywords[index] = keyword;
        keyword
    }

    /// Look up a pre-interned identifier at a token index.
    #[inline]
    pub(crate) fn identifier_for_index(&mut self, index: usize) -> Option<StringId> {
        self.token_stream.ensure_token(index);

        if self.token_identifiers.len() <= index {
            self.token_identifiers.resize(index + 1, None);
        }

        if self.token_identifiers[index].is_some() {
            return self.token_identifiers[index];
        }

        let token = self.tokens().get(index)?;
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        let span_str = self.file.span_str(token.span);
        let identifier = Some(self.strings.intern(span_str));
        self.token_identifiers[index] = identifier;
        identifier
    }

    /// Get the token index used by peek_next.
    #[inline]
    pub(crate) fn index_for_next(&self) -> usize {
        if self.has_active_split() {
            self.pos
        } else {
            self.pos + 1
        }
    }

    /// Get the token index used by peek_next_next.
    #[inline]
    pub(crate) fn index_for_next_next(&self) -> usize {
        if self.has_active_split() {
            self.pos + 1
        } else {
            self.pos + 2
        }
    }

    /// Ensure token caches align with the current token stream after a rewind.
    fn reset_token_caches_after_rewind(&mut self) {
        self.truncate_token_caches();
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
        let start = self.mark();
        let mut expressions = self.with_recovery(
            &start,
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
    fn has_annotation_tokens(&mut self) -> bool {
        self.lex_to_end();
        self.side_tokens().iter().any(|token| {
            matches!(
                token.token.ty,
                TokenType::LineComment
                    | TokenType::DocLineComment
                    | TokenType::BlockComment
                    | TokenType::DocBlockComment
            )
        })
    }

    /// Check if there are any blank annotations (multiple newlines) in main tokens.
    fn has_blank_annotation_tokens(&mut self) -> bool {
        self.token_stream.lex_to_end();
        let mut seen_newline = false;
        for token in self.tokens() {
            let token_ty = token.token.ty;
            if token_ty == TokenType::Whitespace {
                continue;
            }
            if token_ty == TokenType::Newline {
                if seen_newline {
                    return true;
                }
                seen_newline = true;
                continue;
            }
            seen_newline = false;
        }

        false
    }

    /// Return true when annotations should be attached.
    pub(crate) fn should_attach_annotations(&mut self) -> bool {
        if self.has_annotation_tokens() {
            return true;
        }
        if self.has_blank_annotation_tokens() {
            return true;
        }

        !self.tree.get_nodes::<Decorator>().is_empty()
    }

    /// Get a span covering the entire file.
    fn file_span(&self) -> Span {
        Span::new(self.file_id, 0, self.file.len)
    }

    /// Finish parsing. You don't need to call this manually if using Parser::parse().
    pub fn finish(&mut self) {
        if !self.is_finished {
            self.finish_annotations();
            self.finish_positions();
            self.is_finished = true;
        }
    }

    /// Attach annotation nodes after parsing.
    pub fn finish_annotations(&mut self) {
        if self.is_finished {
            return;
        }
        self.attach_annotations();
    }

    /// Build the position index after parsing.
    pub fn finish_positions(&mut self) {
        if self.is_finished {
            return;
        }
        if self.positions_built {
            return;
        }
        let _timing = self.timing_scope(tags::PARSE_POSITIONS_BUILD);
        self.tree.build_position_index();
        self.positions_built = true;
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
            let diagnostic = e.to_diagnostic(self.file.as_ref(), self.tokens());
            self.diagnostics.insert(diagnostic);
        }
    }

    /// Gets a mark of the current position.
    #[inline]
    pub fn mark(&self) -> ParserMark {
        ParserMark::new(
            self.pos,
            self.split_token,
            self.split_token_consumed,
            self.token_stream.mark(),
        )
    }

    /// Run a closure at a temporary token position and restore parser state afterward.
    pub(crate) fn with_pos<T>(&mut self, pos: usize, func: impl FnOnce(&mut Self) -> T) -> T {
        let mark = self.mark();
        self.pos = pos;
        let result = func(self);
        self.rewind(mark);
        result
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn rewind(&mut self, mark: ParserMark) {
        self.pos = mark.pos;
        self.split_token = mark.split_token;
        self.split_token_consumed = mark.split_token_consumed;
        if let Some(token_stream_mark) = mark.token_stream_mark {
            self.token_stream.restore(token_stream_mark);
            self.reset_token_caches_after_rewind();
        }
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn restore(&mut self, mark: ParserMark, idx: u32) {
        self.pos = mark.pos;
        self.split_token = mark.split_token;
        self.split_token_consumed = mark.split_token_consumed;
        self.tree.reset_to(idx);
        if let Some(token_stream_mark) = mark.token_stream_mark {
            self.token_stream.restore(token_stream_mark);
            self.reset_token_caches_after_rewind();
        }
    }

    /// Get a mark and return the span of the current position.
    #[inline]
    pub fn get_span_from(&self, mark: &ParserMark) -> Span {
        if let Some(span) = mark.span_override {
            return span;
        }

        // if we're beyond the end we just point to the EOF token
        if mark.pos >= self.tokens().len() {
            return self.eof_span();
        }

        // otherwise, get the span from the token
        let start_token = self.tokens()[mark.pos];
        let end_token = if self.pos > 0 {
            self.tokens()[self.pos - 1]
        } else {
            self.tokens()[0]
        };
        Span {
            file: self.file_id,
            start: start_token.span.start,
            end: end_token.span.end,
        }
    }

    /// Get the span between two marks.
    #[inline]
    pub fn get_span_between(&self, start: &ParserMark, end: &ParserMark) -> Span {
        let start_token = self.tokens()[start.pos];
        let end_token = self.tokens()[end.pos];
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
            self.tokens().get(self.pos - 1)
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
    pub fn peek(&mut self) -> ParseResult<&TokenSpan> {
        // return split token if present and not yet consumed
        if let Some(ref split) = self.split_token
            && !self.split_token_consumed
        {
            return Ok(split);
        }

        self.token_stream.ensure_token(self.pos);
        self.tokens()
            .get(self.pos)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next token type, defaulting to End at EOF.
    #[inline]
    pub fn peek_token_type(&mut self) -> TokenType {
        if let Some(split) = self.split_token
            && !self.split_token_consumed
        {
            return split.token.ty;
        }

        self.token_stream.ensure_token(self.pos);
        self.tokens()
            .get(self.pos)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Peek the next token type, skipping an active split token.
    #[inline]
    pub fn peek_next_token_type(&mut self) -> TokenType {
        let has_active_split = self.split_token.is_some() && !self.split_token_consumed;
        let offset = if has_active_split { 0 } else { 1 };

        self.token_stream.ensure_token(self.pos + offset);
        self.tokens()
            .get(self.pos + offset)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Return true when the next token matches the given type.
    #[inline]
    pub fn peek_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_is requires semantic token type"
        );
        self.peek_token_type() == token_type
    }

    /// Return true when the next-next token matches the given type.
    #[inline]
    pub fn peek_next_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_is requires semantic token type"
        );
        self.peek_next_token_type() == token_type
    }

    /// Return true when more tokens remain before End.
    #[inline]
    pub fn has_more_tokens(&mut self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&mut self) -> ParseResult<&TokenSpan> {
        // if split_token is active (present and not consumed), peek_next looks at current position
        let has_active_split = self.split_token.is_some() && !self.split_token_consumed;
        let offset = if has_active_split { 0 } else { 1 };

        self.token_stream.ensure_token(self.pos + offset);
        self.tokens()
            .get(self.pos + offset)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&mut self) -> ParseResult<&TokenSpan> {
        // if split_token is active, offset by one less
        let has_active_split = self.split_token.is_some() && !self.split_token_consumed;
        let offset = if has_active_split { 1 } else { 2 };

        self.token_stream.ensure_token(self.pos + offset);
        self.tokens()
            .get(self.pos + offset)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next next next Token or error.
    #[inline]
    pub fn peek_next_next_next(&mut self) -> ParseResult<&TokenSpan> {
        // if split_token is active, offset by one less
        let has_active_split = self.split_token.is_some() && !self.split_token_consumed;
        let offset = if has_active_split { 2 } else { 3 };

        self.token_stream.ensure_token(self.pos + offset);
        self.tokens()
            .get(self.pos + offset)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParseResult<&TokenSpan> {
        // if there's an active split token, consume it and return reference
        if let Some(ref split) = self.split_token
            && !self.split_token_consumed
        {
            self.split_token_consumed = true;
            return Ok(split);
        }
        // normal case: consume from token stream
        self.token_stream.ensure_token(self.pos);
        if self.pos < self.tokens().len() {
            let pos = self.pos;
            self.pos += 1;
            self.tokens()
                .get(pos)
                .ok_or(ParseError::unexpected(self.eof_span()))
        } else {
            Err(ParseError::unexpected(self.eof_span()))
        }
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        // if there's an active split token, mark it as consumed instead of advancing
        if self.split_token.is_some() && !self.split_token_consumed {
            self.split_token_consumed = true;
            return;
        }

        self.token_stream.ensure_token(self.pos);
        debug_assert!(self.pos < self.tokens().len(), "bump past end of tokens");
        self.pos += 1;
    }

    /// Bump the Token position by a given distance.
    #[inline]
    pub fn bump_by(&mut self, distance: u8) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.token_stream
            .ensure_token(self.pos + (distance as usize));
        debug_assert!(
            self.pos + (distance as usize) < self.tokens().len(),
            "bump past end of tokens"
        );
        self.pos += distance as usize;
    }

    /// Advance the token position to a specific index.
    #[inline]
    pub(crate) fn advance_to(&mut self, pos: usize) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.token_stream.ensure_token(pos);
        debug_assert!(pos <= self.tokens().len(), "advance past end of tokens");
        self.pos = pos;
    }

    /// Split a `<<` (ShiftLeft) token into two `<` tokens.
    /// Consumes the ShiftLeft and stores a synthetic `<` as the pending split token.
    /// Used when `<<` needs to become `<` + `<` in generic contexts like `Extends<<T>()...>`.
    pub fn split_shift_left(&mut self) {
        self.token_stream.ensure_token(self.pos);
        let current = &self.tokens()[self.pos];
        debug_assert_eq!(
            current.token.ty,
            TokenType::ShiftLeft,
            "split_shift_left called on non-ShiftLeft token"
        );

        // span for the second `<` (offset by 1 character)
        let second_span = Span {
            file: current.span.file,
            start: current.span.start + 1,
            end: current.span.end,
        };

        // store synthetic `<` token as pending
        self.split_token = Some(TokenSpan {
            token: Token {
                ty: TokenType::LessThan,
                len: 1,
                literal: None,
            },
            span: second_span,
        });
        self.split_token_consumed = false;

        // advance past the ShiftLeft token
        self.pos += 1;
    }

    /// Eat a single `>` token in generic close contexts.
    ///
    /// This handles glued operator tails like `>=`, `>>=`, and `>>>=`
    /// by consuming one `>` and leaving the remainder as a pending split token.
    pub fn eat_type_angle_close(&mut self) -> ParseResult<()> {
        if self.peek_is(TokenType::GreaterThan) {
            self.bump();
            return Ok(());
        }

        let token = *self.peek()?;
        let split_type = match token.token.ty {
            TokenType::GreaterThanOrEqual => TokenType::Assign,
            TokenType::ShiftRightAssign => TokenType::GreaterThanOrEqual,
            TokenType::UnsignedShiftRightAssign => TokenType::ShiftRightAssign,
            _ => return Err(ParseError::unexpected(token.span)),
        };

        let split_span = Span {
            file: token.span.file,
            start: token.span.start + 1,
            end: token.span.end,
        };

        self.split_token = Some(TokenSpan {
            token: Token {
                ty: split_type,
                len: token.token.len.saturating_sub(1),
                literal: None,
            },
            span: split_span,
        });
        self.split_token_consumed = false;
        self.pos += 1;
        Ok(())
    }

    /// Check if there's an active (unconsumed) split token of the given type.
    #[inline]
    pub fn has_split_token(&self, token_type: TokenType) -> bool {
        self.split_token
            .map(|t| t.token.ty == token_type && !self.split_token_consumed)
            .unwrap_or(false)
    }

    /// Peek a token at a position.
    #[inline]
    pub fn peek_token_ahead(
        &mut self,
        delta: u32,
        token_type: TokenType,
    ) -> ParseResult<&TokenSpan> {
        let index = self.pos + (delta as usize);
        self.token_stream.ensure_token(index);
        self.tokens()
            .get(index)
            .filter(|token| token.token.ty == token_type)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
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
    pub fn peek_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
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
    pub fn peek_next_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next token in a list of token types.
    #[inline]
    pub fn peek_next_next_token_in(
        &mut self,
        token_types: &[TokenType],
    ) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
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
        &mut self,
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
        if self.peek_is(token_type) {
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
        start: &ParserMark,
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
        start: &ParserMark,
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
        if self.peek_is(expected) {
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
                let error = ParseError::unexpected(self.get_span_from(&start));
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
        let error = ParseError::unexpected(self.get_span_from(&start));
        self.error(&error);
        Err(error)
    }

    /// Get the node starting at a token.
    pub fn find_node_starting_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            |candidate| candidate.span.start == span.start,
        )
    }

    /// Get the node ending at a token.
    pub fn find_node_ending_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            |candidate| candidate.span.end == span.end,
        )
    }

    /// Get the node enclosing a token.
    pub fn find_node_enclosing_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            filter,
        )
    }

    /// Select the best enclosing span for a given span range and filter.
    fn select_enclosing_span_with_filter(
        &self,
        start: u32,
        end_inclusive: u32,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        let mut best = None;
        self.tree
            .source_map
            .visit_enclosing_spans(start, end_inclusive, |candidate| {
                if !filter(&candidate) {
                    return;
                }
                match best {
                    Some(current) => {
                        if self.is_better_enclosing_span(search, &candidate, &current) {
                            best = Some(candidate);
                        }
                    }
                    None => {
                        best = Some(candidate);
                    }
                }
            });
        best
    }

    /// Return true when candidate outranks current for the search mode.
    fn is_better_enclosing_span(
        &self,
        search: NodeSearchMode,
        candidate: &EnclosingSpan,
        current: &EnclosingSpan,
    ) -> bool {
        let candidate_len = candidate.length;
        let current_len = current.length;
        let candidate_idx = candidate.idx;
        let current_idx = current.idx;

        match search {
            NodeSearchMode::BiggestOutermost => {
                candidate_len > current_len
                    || (candidate_len == current_len && candidate_idx > current_idx)
            }
            NodeSearchMode::SmallestOutermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_idx > current_idx)
            }
            NodeSearchMode::SmallestInnermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_idx < current_idx)
            }
        }
    }

    /// Check if two spans are on the same line.
    #[inline]
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        self.file.is_same_line(left.start, right.end)
    }
}

fn timings_enabled_from_env() -> bool {
    std::env::var("DESTACK_PARSER_TIMINGS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .or_else(|| {
            std::env::var("DESTACK_TIMINGS")
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .map(|value| value > 0)
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
    /// The split token at the time of marking (for restoring on rewind).
    split_token: Option<TokenSpan>,
    /// Whether the split token was consumed at mark time.
    split_token_consumed: bool,
    /// The token stream mark for speculative parsing.
    token_stream_mark: Option<TokenStreamMark>,
    /// Optional override span for synthetic marks.
    span_override: Option<Span>,
}

impl ParserMark {
    /// Create a new ParserMark.
    #[inline]
    pub(crate) fn new(
        pos: usize,
        split_token: Option<TokenSpan>,
        split_token_consumed: bool,
        token_stream_mark: TokenStreamMark,
    ) -> Self {
        Self {
            pos,
            split_token,
            split_token_consumed,
            token_stream_mark: Some(token_stream_mark),
            span_override: None,
        }
    }

    /// Create a synthetic mark from a span without capturing token stream state.
    #[inline]
    pub(crate) fn from_span(span: Span) -> Self {
        Self {
            pos: 0,
            split_token: None,
            split_token_consumed: false,
            token_stream_mark: None,
            span_override: Some(span),
        }
    }
}
