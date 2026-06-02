use crate::{Lexer, LexerCheckpoint, ParserTriviaMode, is_semantic, keyword_from_identifier};
use core::fmt;
use destack_core::StringPool;
use destack_dir::{
    BlockForm, Comment, Expression, Keyword, LocalNodeId, Node, NodeType, Token, TokenLiteral,
    TokenRange, TokenSpan, TokenType, Tree, TreeCapacity, TreeMark, TreeStore,
};
use destack_source::{
    Diagnostic, DiagnosticCollection, EnclosingSpan, File, FileId, LanguageType, ModuleId,
    MultiSpan, NodeSearchMode, NodeSpanBoundary, NodeSpanRegion, NodeSpanType, PackageId, Span,
};
use std::collections::HashSet;
use std::fmt::Debug;
use std::mem;
use std::sync::Arc;

use crate::{ParserError, ParserResult};

use super::flags::ParserFlags;
use super::mode::ContextualLexMode;
use super::options::ParserOptions;

/// Estimated source bytes per parser token.
const ESTIMATED_TOKEN_BYTES: usize = 6;
/// Estimated parser tokens per interned string.
const ESTIMATED_STRING_TOKEN_DIVISOR: usize = 3;
/// Maximum nested recursive parser descent before reporting malformed input.
const MAX_RECURSIVE_DESCENT_DEPTH: u16 = 2048;

/// Stable parser error identity used for diagnostic deduplication.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
struct ParserErrorKey {
    /// The leaf error span.
    span: Span,
    /// The leaf parser node type.
    node_type: Option<NodeType>,
    /// The expected token at the leaf.
    expected: Option<TokenType>,
}

impl ParserErrorKey {
    /// Create one deduplication key from one parser error.
    fn from_error(error: &ParserError) -> Self {
        let (span, node_type, expected) = error.leaf_content();

        Self {
            span,
            node_type,
            expected,
        }
    }
}

/// One cached parser lookahead token.
#[derive(Debug, Copy, Clone)]
struct ParserLookaheadToken {
    /// The current token span that owns the cache.
    current_span: Span,
    /// The contextual lexing mode used to produce the lookahead token.
    mode: ContextualLexMode,
    /// The lookahead token.
    token: TokenSpan,
}

/// A parser for a single source file.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser {
    /// The source we're parsing.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The live lexer cursor.
    lexer: Lexer,
    /// The tokens consumed by parser context-sensitive interpretation.
    consumed_tokens: Vec<TokenRange>,
    /// The side token stream.
    side_tokens: Vec<TokenRange>,
    /// The structured comments collected during lexing.
    comments: Vec<Comment>,
    /// The parser trivia retention mode.
    trivia_mode: ParserTriviaMode,
    /// Whether tree literal token interpretation is enabled.
    allow_tree_literals: bool,
    /// The lexing mode for the next token read.
    contextual_lex_mode: ContextualLexMode,

    /// The current visible token at the parser cursor.
    current_token: TokenSpan,
    /// The previous semantic token end.
    previous_token_end: u32,
    /// The last consumed visible token.
    last_consumed_token: TokenSpan,
    /// The cached next visible token.
    next_token_cache: Option<ParserLookaheadToken>,
    /// Whether the parser is finished.
    is_finished: bool,
    /// The parser context flags.
    pub(crate) flags: ParserFlags,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    preserve_parenthesized_wrappers: bool,
    /// The current nested recursive descent depth.
    recursive_descent_depth: u16,

    /// The Node DIR tree.
    pub tree: Tree,
    /// The shared string pool.
    pub strings: Arc<StringPool>,

    /// The language type for parsing behavior.
    pub language: LanguageType,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParserError>,
    /// The parser error keys encountered so far.
    error_keys: HashSet<ParserErrorKey>,
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Return true when the current token starts after a line break.
    #[inline]
    pub(crate) fn current_token_is_on_new_line(&self) -> bool {
        self.current_token.token.is_on_new_line()
    }

    /// Return true when source trivia before the offset token contains a line break.
    #[inline]
    pub(crate) fn token_at_offset_has_leading_line_break(&mut self, offset: usize) -> bool {
        self.token_at_offset(offset).token.is_on_new_line()
    }

    /// Return true when comments appear between the previous token and current token.
    pub(crate) fn current_token_has_leading_comment(&self) -> bool {
        self.source_range_has_comment(self.previous_token_end, self.current_token.span.start)
    }

    /// Return true when comments appear before one peeked token.
    pub(crate) fn token_has_leading_comment_after(
        &self,
        previous_end: u32,
        token: TokenSpan,
    ) -> bool {
        self.source_range_has_comment(previous_end, token.span.start)
    }

    /// Return true when one source range contains a line or block comment.
    fn source_range_has_comment(&self, start: u32, end: u32) -> bool {
        let bytes = self.file.text().as_bytes();
        let mut offset = start as usize;
        let end = end as usize;

        // scan trivia bytes between visible tokens
        while offset + 1 < end {
            if bytes[offset] == b'/' && matches!(bytes[offset + 1], b'/' | b'*') {
                return true;
            }

            offset += 1;
        }

        false
    }

    /// Return true when transparent parenthesized wrappers stay in the parsed tree.
    #[inline]
    pub(crate) fn preserves_parenthesized_wrappers(&self) -> bool {
        self.preserve_parenthesized_wrappers
    }

    /// Run one parser descent under the shared recursion limit.
    #[inline(always)]
    pub(crate) fn with_recursive_descent<T>(
        &mut self,
        owner: NodeType,
        parse: impl FnOnce(&mut Self) -> ParserResult<T>,
    ) -> ParserResult<T> {
        if self.recursive_descent_depth >= MAX_RECURSIVE_DESCENT_DEPTH {
            return Err(ParserError::unexpected_for(self.peek()?.span, owner));
        }

        self.recursive_descent_depth += 1;
        let result = destack_core::ensure_sufficient_stack(|| parse(self));
        self.recursive_descent_depth -= 1;

        result
    }

    /// Create one parser that appends into an existing DIR tree.
    fn parser_with_tree(
        file: Arc<File>,
        language: LanguageType,
        strings: Arc<StringPool>,
        tree: Tree,
        options: ParserOptions,
    ) -> Self {
        let source_len = file.text().len();
        let estimated_tokens = source_len / ESTIMATED_TOKEN_BYTES;
        let estimated_side_tokens = if options.trivia_mode.keeps_side_tokens() {
            estimated_tokens / 2
        } else {
            0
        };
        let estimated_comments = if options.trivia_mode.keeps_comments() {
            estimated_tokens / 16
        } else {
            0
        };
        let estimated_strings = estimated_tokens / ESTIMATED_STRING_TOKEN_DIVISOR;

        // create the live lexer cursor
        let mut lexer = Lexer::new(file.clone(), language);
        lexer.set_trivia_mode(options.trivia_mode);

        // initialize source-local parser state
        let file_id = file.id;
        strings.reserve(estimated_strings);
        Self {
            file,
            file_id,
            lexer,
            consumed_tokens: Vec::with_capacity(estimated_tokens),
            side_tokens: Vec::with_capacity(estimated_side_tokens),
            comments: Vec::with_capacity(estimated_comments),
            trivia_mode: options.trivia_mode,
            allow_tree_literals: language.supports_jsx(),
            contextual_lex_mode: ContextualLexMode::Normal,
            current_token: TokenSpan {
                token: Token::end(),
                span: Span::new(file_id, 0, 0),
            },
            previous_token_end: 0,
            last_consumed_token: TokenSpan {
                token: Token::end(),
                span: Span::new(file_id, 0, 0),
            },
            next_token_cache: None,
            is_finished: false,
            flags: ParserFlags::default(),
            preserve_parenthesized_wrappers: options.preserve_parenthesized_wrappers,
            recursive_descent_depth: 0,
            language,
            tree,
            strings,
            errors: Vec::with_capacity(4),
            error_keys: HashSet::with_capacity(4),
        }
    }

    /// Create a new parser from a text File and tokenize it.
    pub fn lex_file(file: Arc<File>, language: LanguageType, strings: Arc<StringPool>) -> Self {
        let options = ParserOptions {
            trivia_mode: ParserTriviaMode::Documentation,
            ..ParserOptions::default()
        };

        Self::lex_file_with_options(file, language, options, strings)
    }

    /// Create a new parser from a module text File and tokenize it.
    pub fn lex_module(
        module_id: ModuleId,
        file: Arc<File>,
        language: LanguageType,
        strings: Arc<StringPool>,
    ) -> Self {
        let options = ParserOptions {
            trivia_mode: ParserTriviaMode::Documentation,
            ..ParserOptions::default()
        };

        Self::lex_module_with_options(module_id, file, language, options, strings)
    }

    /// Lex a file and apply parser options.
    pub fn lex_file_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let module_id = ModuleId::new(PackageId::new(0), file.id.0);

        Self::lex_module_with_options(module_id, file, language, options, strings)
    }

    /// Lex a module text File and apply parser options.
    pub fn lex_module_with_options(
        module_id: ModuleId,
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let source_len = file.text().len();
        let estimated_tokens = source_len / ESTIMATED_TOKEN_BYTES;
        let tree = Tree::with_capacities(module_id, Self::estimate_tree_capacity(estimated_tokens));

        Self::lex_module_tree_with_options(file, language, options, strings, tree)
    }

    /// Estimate initial tree buffers from token count.
    fn estimate_tree_capacity(estimated_tokens: usize) -> TreeCapacity {
        TreeCapacity {
            nodes: estimated_tokens,
            comments: estimated_tokens / 16,
            ..TreeCapacity::default()
        }
    }

    /// Lex a module text File into an existing DIR tree and apply parser options.
    pub fn lex_module_tree_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
        tree: Tree,
    ) -> Self {
        let mut parser = Self::parser_with_tree(file, language, strings, tree, options);
        parser.reset();
        parser.apply_options(options);
        parser
    }

    /// Apply externally provided parser options.
    #[inline]
    pub fn apply_options(&mut self, options: ParserOptions) {
        self.flags
            .set_disallow_ambiguous_tree_literal(options.disallow_ambiguous_tree_literal);
        self.preserve_parenthesized_wrappers = options.preserve_parenthesized_wrappers;
        debug_assert!(
            self.trivia_mode == options.trivia_mode,
            "trivia retention must be configured before lexing starts"
        );
    }

    /// Get the span of all side annotations.
    #[inline]
    pub fn compute_side_span(&self) -> MultiSpan {
        Self::compute_side_span_from_tree(&self.tree)
    }

    /// Get the span of all side decorators from a tree.
    #[inline]
    pub fn compute_side_span_from_tree(tree: &Tree) -> MultiSpan {
        MultiSpan::new(tree.get_side_decorator_spans())
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.previous_token_end = 0;
        self.consumed_tokens.clear();
        self.side_tokens.clear();
        self.comments.clear();
        self.lexer = Lexer::new(self.file.clone(), self.language);
        self.lexer.set_trivia_mode(self.trivia_mode);
        self.contextual_lex_mode = ContextualLexMode::Normal;
        self.allow_tree_literals = self.language.supports_jsx();
        let mut flags = ParserFlags::default();
        flags.set_disallow_ambiguous_tree_literal(
            self.language.supports_jsx() && self.language.is_typescript(),
        );
        self.flags = flags;
        self.errors.clear();
        self.error_keys.clear();

        self.read_next_token();
        self.previous_token_end = 0;
        self.last_consumed_token = TokenSpan {
            token: Token::end(),
            span: Span::new(self.file_id, 0, 0),
        };
        self.next_token_cache = None;
    }

    /// Swap parser flags and return the previous value.
    #[inline(always)]
    pub(crate) fn swap_flags(&mut self, flags: ParserFlags) -> ParserFlags {
        let old_flags = self.flags;
        self.flags = flags;
        old_flags
    }

    /// Restore parser flags from a previous swap.
    #[inline(always)]
    pub(crate) fn restore_flags(&mut self, old_flags: ParserFlags) {
        self.flags = old_flags;
    }

    /// Execute a function with new parser flags.
    /// The previous flags are restored after the function returns.
    #[inline(always)]
    pub(crate) fn with_flags<T>(
        &mut self,
        flags: ParserFlags,
        func: impl FnOnce(&mut Self) -> T,
    ) -> T {
        if self.flags == flags {
            func(self)
        } else {
            let old_flags = self.swap_flags(flags);
            let result = func(self);
            self.restore_flags(old_flags);

            result
        }
    }

    /// Return the current semantic tokens.
    #[inline]
    pub(crate) fn tokens(&self) -> Vec<TokenSpan> {
        self.consumed_tokens
            .iter()
            .copied()
            .map(|token| token.with_file(self.file_id))
            .collect()
    }

    /// Return the innermost expression after skipping parenthesized wrappers.
    #[inline]
    pub(crate) fn without_parentheses_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        while let Expression::Parenthesized { expression } = self.tree.get(expression_id) {
            expression_id = *expression;
        }

        expression_id
    }

    /// Return true when tree literal lexing is enabled.
    #[inline]
    pub(crate) fn allow_tree_literals(&self) -> bool {
        self.allow_tree_literals
    }

    /// Set whether tree literal lexing is enabled.
    #[inline]
    pub(crate) fn set_allow_tree_literals(&mut self, allow: bool) {
        self.allow_tree_literals = allow;
    }

    /// Eat a tree opening `<`.
    #[inline]
    pub(crate) fn eat_tree_opening_angle(&mut self) -> ParserResult<()> {
        if !self.peek_is(TokenType::LessThan) {
            return Err(ParserError::expected(
                self.peek()?.span,
                TokenType::LessThan,
            ));
        }

        self.bump_tree_opening_angle();
        Ok(())
    }

    /// Advance past a tree opening `<`.
    #[inline]
    fn bump_tree_opening_angle(&mut self) {
        self.contextual_lex_mode = ContextualLexMode::TreeTag;
        self.next_token_cache = None;
        self.bump();
    }

    /// Enable or disable tree attribute value lexing for the next token.
    #[inline]
    pub(crate) fn set_tree_attribute_value(&mut self, enabled: bool) {
        self.contextual_lex_mode = if enabled {
            ContextualLexMode::TreeAttributeValue
        } else {
            ContextualLexMode::Normal
        };
        self.next_token_cache = None;
    }

    /// Read the next token in tree tag mode.
    #[inline]
    pub(crate) fn set_tree_tag_follow(&mut self) {
        self.contextual_lex_mode = ContextualLexMode::TreeTag;
        self.next_token_cache = None;
    }

    /// Bump the current token and read the next one in a contextual lexer mode.
    #[inline]
    pub(crate) fn bump_with_contextual_lex_mode(&mut self, mode: ContextualLexMode) {
        self.contextual_lex_mode = mode;
        self.next_token_cache = None;
        self.drop_side_tokens_covered_by_current();
        self.bump();
    }

    /// Re-lex the current token as a generic `<`.
    #[inline]
    pub(crate) fn re_lex_generic_l_angle(&mut self) -> bool {
        let token_type = self.current_token.token.ty();
        if token_type == TokenType::LessThan {
            return true;
        }

        if !matches!(
            token_type,
            TokenType::ShiftLeft | TokenType::LessThanOrEqual | TokenType::ShiftLeftAssign
        ) {
            return false;
        }

        let token = self.split_current_token_prefix(TokenType::LessThan, 1);
        self.current_token = token;
        true
    }

    /// Re-lex the current token as one `>`.
    #[inline]
    pub(crate) fn re_lex_r_angle(&mut self) -> bool {
        let token_type = self.current_token.token.ty();
        if token_type == TokenType::GreaterThan {
            return true;
        }

        if !matches!(
            token_type,
            TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            return false;
        }

        let token = self.split_current_token_prefix(TokenType::GreaterThan, 1);
        self.current_token = token;
        true
    }

    /// Re-lex the current `/` or `/=` token as a regex literal
    #[inline]
    pub(crate) fn re_lex_regex(&mut self) -> bool {
        let token_type = self.current_token.token.ty();
        if token_type == TokenType::Literal
            && matches!(
                self.current_token.token.literal(),
                Some(TokenLiteral::RegexString { .. })
            )
        {
            return true;
        }

        if !matches!(token_type, TokenType::Divide | TokenType::DivideAssign) {
            return false;
        }

        let token = self.reclassify_current_divide_as_regex();
        self.current_token = token;
        true
    }

    /// Split the current compound token and keep one prefix token at the cursor.
    fn split_current_token_prefix(&mut self, token_type: TokenType, prefix_len: u32) -> TokenSpan {
        self.next_token_cache = None;
        let current = self.current_token;
        debug_assert!(prefix_len > 0 && prefix_len <= current.token.len());

        let prefix = TokenSpan {
            token: Token::new(token_type, prefix_len, None)
                .with_on_new_line(current.token.is_on_new_line()),
            span: Span::new(
                current.span.file,
                current.span.start,
                current.span.start + prefix_len,
            ),
        };

        if current.token.len() > prefix_len {
            let rest_start = current.span.start + prefix_len;
            self.lexer.set_position(rest_start as usize);
        }

        prefix
    }

    /// Reclassify the current slash token as one regex literal token.
    fn reclassify_current_divide_as_regex(&mut self) -> TokenSpan {
        self.next_token_cache = None;
        let current = self.current_token;
        let source = self.file.text();
        let mut index = current.span.start as usize + 1;
        let mut escaped = false;
        let mut in_character_class = false;

        while index < source.len() {
            let Some(character) = source[index..].chars().next() else {
                break;
            };
            index += character.len_utf8();

            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if in_character_class {
                if character == ']' {
                    in_character_class = false;
                }
                continue;
            }
            if character == '[' {
                in_character_class = true;
                continue;
            }
            if character == '/' {
                break;
            }
        }

        let mut has_flags = false;
        while index < source.len() {
            let Some(character) = source[index..].chars().next() else {
                break;
            };
            if !character.is_ascii_alphabetic() {
                break;
            }
            has_flags = true;
            index += character.len_utf8();
        }

        let end = index as u32;
        self.lexer.set_position(end as usize);

        TokenSpan {
            token: Token::new(
                TokenType::Literal,
                end - current.span.start,
                Some(TokenLiteral::RegexString { has_flags }),
            )
            .with_on_new_line(current.token.is_on_new_line()),
            span: Span::new(current.span.file, current.span.start, end),
        }
    }

    /// Eat one typed angle-close token.
    #[inline]
    pub(crate) fn eat_type_angle_close(&mut self) -> ParserResult<()> {
        if !self.re_lex_r_angle() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.bump();
        Ok(())
    }

    /// Eat one expression-position typed angle-close token.
    #[inline]
    pub(crate) fn eat_expression_type_angle_close(&mut self) -> ParserResult<()> {
        if !Self::starts_expression_type_angle_close(self.peek_token_type()) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.eat_type_angle_close()
    }

    /// Return true when one token can begin a type-angle close sequence.
    #[inline]
    pub(crate) const fn starts_type_angle_close(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        )
    }

    /// Return true when one token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) const fn starts_expression_type_angle_close(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan | TokenType::ShiftRight | TokenType::UnsignedShiftRight
        )
    }

    /// Return true when the current token can begin a type-angle close sequence.
    #[inline]
    pub(crate) fn peek_starts_type_angle_close(&mut self) -> bool {
        Self::starts_type_angle_close(self.peek_token_type())
    }

    /// Return true when the current token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) fn peek_starts_expression_type_angle_close(&mut self) -> bool {
        Self::starts_expression_type_angle_close(self.peek_token_type())
    }

    /// Return true when the current token can begin one `>` in tree tag syntax.
    #[inline]
    pub(crate) fn peek_starts_tree_tag_close(&mut self) -> bool {
        Self::starts_type_angle_close(self.peek_token_type())
    }

    /// Return owned token buffers after lexing to EOF.
    pub fn take_tokens(&mut self) -> (Vec<TokenRange>, Vec<TokenRange>) {
        let mut tokens = mem::take(&mut self.consumed_tokens);
        tokens.push(TokenRange::from_token_span(self.current_token));

        if self.current_token.token.ty() == TokenType::End {
            self.drain_lexer_side_tokens();
            tokens.shrink_to_fit();
            self.side_tokens.shrink_to_fit();

            return (tokens, mem::take(&mut self.side_tokens));
        }

        self.contextual_lex_mode = ContextualLexMode::Normal;
        loop {
            let token = self.lexer.next_semantic_token();
            let is_end = token.token.ty() == TokenType::End;
            tokens.push(TokenRange::from_token_span(token));

            if is_end {
                break;
            }
        }
        self.drain_lexer_side_tokens();
        tokens.shrink_to_fit();
        self.side_tokens.shrink_to_fit();

        (tokens, mem::take(&mut self.side_tokens))
    }

    /// Return owned full token span buffers after lexing to EOF.
    pub fn take_token_spans(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        let file_id = self.file_id;
        let (tokens, side_tokens) = self.take_tokens();
        let tokens = tokens
            .into_iter()
            .map(|token| token.with_file(file_id))
            .collect();
        let side_tokens = side_tokens
            .into_iter()
            .map(|token| token.with_file(file_id))
            .collect();

        (tokens, side_tokens)
    }

    /// Return the EOF span without forcing a full lex.
    #[inline]
    pub(crate) fn eof_span(&self) -> Span {
        Span::new(self.file_id, self.file.len, self.file.len)
    }

    /// Return one visible token without moving the parser cursor.
    #[inline(always)]
    fn cached_next_token(&self) -> Option<TokenSpan> {
        let cached = self.next_token_cache.as_ref()?;
        let cache_matches_current = cached.current_span == self.current_token.span;
        let cache_matches_mode = cached.mode == self.contextual_lex_mode;

        (cache_matches_current && cache_matches_mode).then_some(cached.token)
    }

    /// Return one visible token without moving the parser cursor.
    pub(crate) fn token_at_offset(&mut self, offset: usize) -> TokenSpan {
        if offset == 0 {
            return self.current_token;
        }

        if offset == 1
            && let Some(token) = self.cached_next_token()
        {
            return token;
        }

        let lexer_checkpoint = self.lexer.checkpoint();
        let contextual_lex_mode = self.contextual_lex_mode;
        let side_tokens_len = self.side_tokens.len();
        let mut token = self.current_token;
        self.lexer.set_cursor_trivia_mode(ParserTriviaMode::Ignore);

        for _ in 0..offset {
            token = self.read_token_from_lexer();
        }

        self.lexer.restore(lexer_checkpoint);
        self.contextual_lex_mode = contextual_lex_mode;
        self.side_tokens.truncate(side_tokens_len);

        if offset == 1 {
            self.next_token_cache = Some(ParserLookaheadToken {
                current_span: self.current_token.span,
                mode: contextual_lex_mode,
                token,
            });
        }

        token
    }

    /// Return one visible token type without moving the parser cursor.
    #[inline(always)]
    pub(crate) fn token_type_at_offset(&mut self, offset: usize) -> TokenType {
        if offset == 0 {
            return self.current_token.token.ty();
        }

        if offset == 1
            && let Some(token) = self.cached_next_token()
        {
            return token.token.ty();
        }

        self.token_at_offset(offset).token.ty()
    }

    /// Return one visible token as a keyword without moving the parser cursor.
    #[inline(always)]
    pub(crate) fn keyword_at_offset(&mut self, offset: usize) -> Option<Keyword> {
        if offset == 0 {
            if self.current_token.token.ty() != TokenType::Identifier {
                return None;
            }

            return keyword_from_identifier(self.get_token_str(self.current_token));
        }

        if offset == 1
            && let Some(token) = self.cached_next_token()
        {
            if token.token.ty() != TokenType::Identifier {
                return None;
            }

            return keyword_from_identifier(self.get_token_str(token));
        }

        let token = self.token_at_offset(offset);
        if token.token.ty() != TokenType::Identifier {
            return None;
        }

        keyword_from_identifier(self.get_token_str(token))
    }

    /// Read the next token from the lexer cursor.
    #[inline]
    fn read_next_token(&mut self) {
        self.next_token_cache = None;
        self.current_token = self.read_token_from_lexer();
    }

    /// Read one token from the live lexer in the current contextual mode.
    fn read_token_from_lexer(&mut self) -> TokenSpan {
        let token = match self.contextual_lex_mode {
            ContextualLexMode::Normal => self.lexer.next_semantic_token(),
            ContextualLexMode::TreeTag => self.lexer.next_tree_tag_token(),
            ContextualLexMode::TreeChild => self.lexer.next_tree_child_token(),
            ContextualLexMode::TreeAttributeValue => self
                .lexer
                .next_tree_attribute_value_token()
                .unwrap_or_else(|| self.lexer.next_semantic_token()),
        };

        if self.contextual_lex_mode == ContextualLexMode::TreeAttributeValue {
            self.contextual_lex_mode = ContextualLexMode::Normal;
        }

        if self.trivia_mode.keeps_side_tokens() {
            self.drain_lexer_side_tokens();
        }

        token
    }

    /// Move produced side tokens into the parser output buffer.
    fn drain_lexer_side_tokens(&mut self) {
        self.lexer
            .drain_side_token_ranges_into(&mut self.side_tokens);
    }

    /// Parse root expressions as an implicit namespace.
    fn parse_roots(&mut self, attach_comments: bool) -> Vec<LocalNodeId<Expression>> {
        let start = self.span_start();
        let mut expressions = self.with_token_recovery(
            &start,
            |parser| parser.eat_block_body(BlockForm::Implicit),
            Vec::new(),
            TokenType::End,
        );
        self.drain_lexer_trivia();

        // ensure one stable owner for trivia only files
        self.ensure_trivia_anchor_maybe(&mut expressions, false);

        // attach comments only in the full parse pipeline
        if attach_comments {
            self.attach_comments();
            self.is_finished = true;
        }

        expressions
    }

    /// Parse everything as an implicit namespace.
    pub fn parse(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_roots(true)
    }

    /// Return whether the parser finished one full parse pipeline.
    pub const fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Parse everything as an implicit namespace without attaching comments.
    pub fn parse_without_attaching_comments(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_roots(false)
    }

    /// Ensure one stable owner for comment trivia in comment only files.
    fn ensure_trivia_anchor_maybe(
        &mut self,
        expressions: &mut Vec<LocalNodeId<Expression>>,
        consumed_to_end: bool,
    ) {
        if !self.trivia_mode.keeps_comments() {
            return;
        }

        // most files already have parsed body expressions and never need a trivia anchor
        if !consumed_to_end && !expressions.is_empty() {
            return;
        }

        // skip files without retained comments
        if self.comments.is_empty() {
            return;
        }

        let stub_span = self.eof_span();

        // comment only files need one returned expression owner
        if expressions.is_empty() {
            let stub = self.insert_node(Expression::Stub, stub_span);
            expressions.push(stub);
            return;
        }

        // only directive only files need an internal trivia owner
        if !consumed_to_end {
            return;
        }

        // directive only files with attachable semantic tokens already have stable owners
        if self
            .consumed_tokens
            .iter()
            .any(|token| token.token.ty() != TokenType::End)
        {
            return;
        }

        // insert one internal anchor so trivia can attach without parse errors
        let _ = self.insert_node(Expression::Stub, stub_span);
    }

    /// Attach retained comments after parsing when needed.
    pub fn attach_comments(&mut self) {
        self.drain_lexer_trivia();

        // skip comment output when trivia retention is disabled
        if !self.trivia_mode.keeps_comments() {
            return;
        }

        if self.comments.is_empty() {
            return;
        }

        // avoid copying comments twice when direct entrypoints attach manually
        if !self.tree.comments().is_empty() {
            return;
        }

        // finalize raw comments in parse order
        let comments = mem::take(&mut self.comments);
        self.tree.comments_mut().extend(comments);
    }

    /// Move produced lexer trivia into parser output buffers.
    fn drain_lexer_trivia(&mut self) {
        self.drain_lexer_side_tokens();
        self.lexer.drain_trivia_comments_into(&mut self.comments);
    }

    /// Handle an error as a Diagnostic.
    /// Errors are deduplicated by leaf content to avoid squiggly red line noise.
    #[cold]
    #[inline(never)]
    pub(crate) fn error(&mut self, e: &ParserError) {
        let key = ParserErrorKey::from_error(e);
        if self.error_keys.insert(key) {
            self.errors.push(e.clone());
        }
    }

    /// Build source diagnostics from parser errors.
    pub fn diagnostics(&self) -> DiagnosticCollection {
        let diagnostics = self
            .errors
            .iter()
            .map(|error| self.diagnostic(error))
            .collect();

        DiagnosticCollection::from_diagnostics(diagnostics)
    }

    /// Build one source diagnostic from one parser error.
    pub fn diagnostic(&self, error: &ParserError) -> Diagnostic {
        let tokens = self.tokens();

        error.to_diagnostic(self.file.as_ref(), &tokens)
    }

    /// Create a checkpoint for speculative parsing that may allocate tree nodes.
    #[inline(always)]
    pub fn checkpoint(&mut self) -> ParserCheckpoint {
        ParserCheckpoint {
            cursor: self.cursor_checkpoint(),
            tree_mark: self.tree.mark(),
            error_count: self.errors.len(),
        }
    }

    /// Create a checkpoint for speculative cursor movement without tree allocation snapshots.
    #[inline(always)]
    pub fn cursor_checkpoint(&mut self) -> ParserCursorCheckpoint {
        ParserCursorCheckpoint {
            lexer_checkpoint: self.lexer.checkpoint(),
            contextual_lex_mode: self.contextual_lex_mode,
            consumed_tokens_len: self.consumed_tokens.len(),
            side_tokens_len: self.side_tokens.len(),
            current_token: self.current_token,
            previous_token_end: self.previous_token_end,
            last_consumed_token: self.last_consumed_token,
        }
    }

    /// Create a lightweight span start at the current parser cursor.
    #[inline(always)]
    pub fn span_start(&self) -> ParserSpanStart {
        ParserSpanStart {
            current_token: self.current_token,
        }
    }

    /// Rewind the parser cursor to one cursor checkpoint.
    pub fn rewind(&mut self, checkpoint: ParserCursorCheckpoint) {
        self.lexer.restore(checkpoint.lexer_checkpoint);
        self.contextual_lex_mode = checkpoint.contextual_lex_mode;
        self.consumed_tokens
            .truncate(checkpoint.consumed_tokens_len);
        self.side_tokens.truncate(checkpoint.side_tokens_len);
        self.current_token = checkpoint.current_token;
        self.previous_token_end = checkpoint.previous_token_end;
        self.last_consumed_token = checkpoint.last_consumed_token;
        self.next_token_cache = None;
    }

    /// Restore the parser and tree to one full checkpoint.
    pub fn restore(&mut self, checkpoint: ParserCheckpoint, source_id: u32) {
        self.rewind(checkpoint.cursor);
        debug_assert_eq!(checkpoint.tree_mark.next_global_id(), source_id);
        self.tree.restore_to_mark(checkpoint.tree_mark);
        self.restore_errors(checkpoint.error_count);
    }

    /// Restore parser errors to one checkpoint.
    fn restore_errors(&mut self, error_count: usize) {
        for error in self.errors.drain(error_count..) {
            let key = ParserErrorKey::from_error(&error);
            self.error_keys.remove(&key);
        }
    }

    /// Run a closure against a speculative parser cursor.
    pub(crate) fn lookahead<T>(&mut self, func: impl FnOnce(&mut Self) -> T) -> T {
        let checkpoint = self.cursor_checkpoint();
        self.lexer.set_cursor_trivia_mode(ParserTriviaMode::Ignore);
        let result = func(self);
        self.rewind(checkpoint);

        result
    }

    /// Return the next parser token without consuming it.
    #[inline]
    pub(crate) fn next_token(&mut self) -> TokenSpan {
        self.token_at_offset(1)
    }

    /// Return the next parser token type without consuming it.
    #[inline]
    pub(crate) fn next_token_type(&mut self) -> TokenType {
        self.token_type_at_offset(1)
    }

    /// Return the next parser keyword without consuming it.
    #[inline]
    pub(crate) fn next_keyword(&mut self) -> Option<Keyword> {
        self.keyword_at_offset(1)
    }

    /// Insert a node into the DIR tree.
    #[inline]
    pub(crate) fn insert_node<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.tree.insert_during_parse(node, span)
    }

    /// Attach one child-owned leading boundary span.
    pub(crate) fn set_node_leading_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_start: u32)
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_start >= node_span.start {
            return;
        }

        let leading_span = Span::new(node_span.file, boundary_start, node_span.start);
        self.tree.set_side_span(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            leading_span,
        );
    }

    /// Attach one child-owned trailing boundary span.
    pub(crate) fn set_node_trailing_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_end: u32)
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_end <= node_span.end {
            return;
        }

        let trailing_span = Span::new(node_span.file, node_span.end, boundary_end);
        self.tree.set_side_span(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
            trailing_span,
        );
    }

    /// Attach one transparent wrapper span owned by a node.
    pub(crate) fn set_node_wrapper_span<T>(&mut self, node_id: LocalNodeId<T>, wrapper_span: Span)
    where
        T: Node,
    {
        let node_id = node_id.id;
        let wrapper_span = self
            .tree
            .get_side_span_by_id(node_id, NodeSpanType::Region(NodeSpanRegion::Wrapper))
            .map_or(wrapper_span, |existing_span| {
                existing_span.merge(wrapper_span)
            });

        self.tree.set_side_span_by_id(
            node_id,
            NodeSpanType::Region(NodeSpanRegion::Wrapper),
            wrapper_span,
        );
    }

    /// Return the span from one parser span start to the previous token.
    #[inline(always)]
    pub fn get_span_from(&self, start: &ParserSpanStart) -> Span {
        let start = start.current_token.span.start;
        let end = self.previous_token_end.max(start);
        Span::new(self.file_id, start, end)
    }

    /// Return the span between two parser span starts.
    #[inline(always)]
    pub fn get_span_between(&self, start: &ParserSpanStart, end: &ParserSpanStart) -> Span {
        Span::new(
            self.file_id,
            start.current_token.span.start,
            end.current_token.span.end,
        )
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

    /// Get the source text for the current token.
    #[inline]
    pub(crate) fn current_token_str(&self) -> &str {
        self.get_token_str(self.current_token)
    }

    /// Get the current token.
    #[inline]
    pub(crate) fn current_token(&self) -> TokenSpan {
        self.current_token
    }

    /// Return the current token as a keyword.
    #[inline]
    pub(crate) fn current_keyword(&self) -> Option<Keyword> {
        if self.current_token.token.ty() != TokenType::Identifier {
            return None;
        }

        keyword_from_identifier(self.current_token_str())
    }

    /// Return true when the current identifier has the expected source text.
    #[inline]
    pub(crate) fn current_identifier_str_is(&self, expected: &str) -> bool {
        self.current_token.token.ty() == TokenType::Identifier
            && self.current_token_str() == expected
    }

    /// Return true when the current identifier is `global`.
    #[inline]
    pub(crate) fn is_global_identifier(&self) -> bool {
        self.current_identifier_str_is("global")
    }

    /// Return true when the current identifier is `module`.
    #[inline]
    pub(crate) fn is_module_identifier(&self) -> bool {
        self.language.supports_module_declaration() && self.current_identifier_str_is("module")
    }

    /// Get the previous Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        (self.previous_token_end > 0).then_some(&self.last_consumed_token)
    }

    /// Get the previous Token type.
    #[inline]
    pub fn prev_token_type(&self) -> TokenType {
        self.prev()
            .map(|token| token.token.ty())
            .unwrap_or(TokenType::End)
    }

    /// Return the end offset of the previously consumed semantic token.
    #[inline]
    pub(crate) fn prev_token_end(&self) -> u32 {
        self.previous_token_end
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&mut self) -> ParserResult<&TokenSpan> {
        Ok(&self.current_token)
    }

    /// Peek the next token type, defaulting to End at EOF.
    #[inline]
    pub fn peek_token_type(&mut self) -> TokenType {
        self.current_token.token.ty()
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

    /// Return true when more tokens remain before End.
    #[inline]
    pub fn has_more_tokens(&mut self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParserResult<&TokenSpan> {
        let consumed = self.current_token;
        self.last_consumed_token = consumed;
        self.previous_token_end = consumed.span.end;
        self.consumed_tokens
            .push(TokenRange::from_token_span(consumed));
        self.read_next_token();

        Ok(&self.last_consumed_token)
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.span.end;
        self.consumed_tokens
            .push(TokenRange::from_token_span(self.current_token));
        self.read_next_token();
    }

    /// Drop trivia tokens that are now part of a virtual tree child token.
    fn drop_side_tokens_covered_by_current(&mut self) {
        let token = self.current_token;
        let is_tree_text = token.token.ty() == TokenType::Literal
            && matches!(
                token.token.literal(),
                Some(TokenLiteral::TreeString)
                    | Some(TokenLiteral::Character {
                        is_html_entity: true,
                        ..
                    })
            );

        if !is_tree_text {
            return;
        }

        self.side_tokens.retain(|side_token| {
            side_token.start < token.span.start || side_token.end() > token.span.end
        });
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&mut self, token_type: TokenType) -> ParserResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek()?;
        if next.token.ty() == token_type {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next.span))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_token_in(&mut self, token_types: &[TokenType]) -> ParserResult<&TokenSpan> {
        let next = self.peek()?;
        if token_types.contains(&next.token.ty()) {
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
        if current.token.ty() == token_type {
            Ok(current)
        } else {
            Err(ParserError::unexpected(current.span))
        }
    }

    /// Eat a token maybe.
    pub fn eat_token_maybe(&mut self, token_type: TokenType) -> ParserResult<bool> {
        if self.peek_is(token_type) {
            self.bump();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Eat one tree tag close token and advance in the requested mode.
    #[inline]
    pub(crate) fn eat_tree_tag_close(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<()> {
        if !self.re_lex_r_angle() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.bump_with_contextual_lex_mode(follow_mode);

        Ok(())
    }

    /// Eat a token in a list of tokens.
    #[inline]
    pub fn eat_token_in(&mut self, token_types: &[TokenType]) -> ParserResult<TokenType> {
        let current = self.eat()?;
        if token_types.contains(&current.token.ty()) {
            Ok(current.token.ty())
        } else {
            Err(ParserError::unexpected(current.span))
        }
    }

    /// Eat a token in a list of tokens maybe.
    #[inline]
    pub fn eat_token_in_maybe(
        &mut self,
        token_types: &[TokenType],
    ) -> ParserResult<Option<TokenType>> {
        let token = *self.peek()?;
        if token_types.contains(&token.token.ty()) {
            self.bump();
            Ok(Some(token.token.ty()))
        } else {
            Ok(None)
        }
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
        self.tree.source_index.visit_enclosing_spans(
            self.file_id,
            start,
            end_inclusive,
            |candidate| {
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
            },
        );
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
        let candidate_source_id = candidate.source_id;
        let current_source_id = current.source_id;

        match search {
            NodeSearchMode::BiggestOutermost => {
                candidate_len > current_len
                    || (candidate_len == current_len && candidate_source_id > current_source_id)
            }
            NodeSearchMode::SmallestOutermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_source_id > current_source_id)
            }
            NodeSearchMode::SmallestInnermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_source_id < current_source_id)
            }
        }
    }

    /// Check if two spans are on the same line.
    #[inline]
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        self.file.is_same_line(left.start, right.end)
    }
}
/// Full parser checkpoint for speculative parses that allocate nodes.
#[derive(Debug)]
pub struct ParserCheckpoint {
    /// The parser cursor checkpoint at parser checkpoint time.
    cursor: ParserCursorCheckpoint,
    /// Tree allocation snapshot at checkpoint time.
    tree_mark: TreeMark,
    /// The parser error count at checkpoint time.
    error_count: usize,
}

/// Parser cursor checkpoint for speculative lookahead without node allocation.
#[derive(Debug)]
pub struct ParserCursorCheckpoint {
    /// The lexer checkpoint at parser checkpoint time.
    lexer_checkpoint: LexerCheckpoint,
    /// The contextual lexing mode at checkpoint time.
    contextual_lex_mode: ContextualLexMode,
    /// The consumed token count at checkpoint time.
    consumed_tokens_len: usize,
    /// The side token count at checkpoint time.
    side_tokens_len: usize,
    /// The parser owned current token at checkpoint time.
    current_token: TokenSpan,
    /// The previous semantic token end at checkpoint time.
    previous_token_end: u32,
    /// The last consumed visible token at checkpoint time.
    last_consumed_token: TokenSpan,
}

/// Lightweight parser position used for span construction.
#[derive(Debug, Copy, Clone)]
pub struct ParserSpanStart {
    /// The parser owned current token at span start time.
    current_token: TokenSpan,
}

impl ParserSpanStart {
    /// Return the token span that started this source span.
    #[inline]
    pub(crate) fn token_span(&self) -> Span {
        self.current_token.span
    }

    /// Return the start of the token that started this span.
    #[inline]
    pub(crate) fn token_start(&self) -> u32 {
        self.current_token.span.start
    }

    /// Return the end of the token that started this span.
    #[inline]
    pub(crate) fn token_end(&self) -> u32 {
        self.current_token.span.end
    }

    /// Return whether this span start is before one token span.
    #[inline]
    pub(crate) fn is_before(&self, span: Span) -> bool {
        self.current_token.span.start < span.start
    }
}
