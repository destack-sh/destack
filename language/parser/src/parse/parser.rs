use crate::{Lexer, LexerState, ParserTriviaMode, is_semantic, keyword_from_identifier};
use core::fmt;
use destack_core::{LocalStringPool, StringId, StringPool, ensure_sufficient_stack};
use destack_dir::{
    BlockForm, Comment, Expression, Keyword, LocalNodeId, Node, NodeType, Token, TokenLiteral,
    TokenSpan, TokenType, Tree, TreeMark, TreeStore,
};
use destack_source::{
    ByteRange, Diagnostic, DiagnosticCollection, EnclosingSpan, File, FileId, LanguageType,
    ModuleId, MultiSpan, NodeSearchMode, NodeSpanBoundary, NodeSpanRegion, NodeSpanType, PackageId,
    Span,
};
use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use std::mem;
use std::sync::Arc;

use crate::{ParserError, ParserResult};

use super::flags::ParserFlags;
use super::mode::ContextualLexMode;
use super::options::ParserOptions;

/// Estimated source bytes per parser token.
const ESTIMATED_TOKEN_BYTES: usize = 6;
/// Recursive descents between nested stack checks.
const STACK_CHECK_INTERVAL: u16 = 32;
/// Maximum nested recursive parser descent before reporting malformed input.
const MAX_RECURSIVE_DESCENT_DEPTH: u16 = 2048;

/// Lexer cursor state attached to a parser token boundary.
#[derive(Debug, Copy, Clone)]
struct LookaheadState {
    /// The live lexer state at this boundary.
    lexer: LexerState,
    /// The side-token buffer length at this boundary.
    side_tokens_len: usize,
}

/// Cached future tokens after the current parser token.
#[derive(Debug)]
struct Lookahead {
    /// The unread future tokens.
    tokens: VecDeque<Token>,
    /// The lexer boundaries between unread future tokens.
    boundaries: VecDeque<LookaheadState>,
    /// The lexer state after the current parser token.
    after_current: Option<LookaheadState>,
}

impl Lookahead {
    /// Create an empty lookahead cache.
    fn new() -> Self {
        Self {
            tokens: VecDeque::new(),
            boundaries: VecDeque::new(),
            after_current: None,
        }
    }

    /// Remove all unread tokens and current state.
    fn clear(&mut self) {
        self.tokens.clear();
        self.boundaries.clear();
        self.after_current = None;
    }

    /// Return one future token by parser-relative offset.
    #[inline(always)]
    fn get(&self, offset: usize) -> Option<Token> {
        self.tokens.get(offset.checked_sub(1)?).copied()
    }

    /// Return the final cached future token.
    #[inline(always)]
    fn last(&self) -> Option<Token> {
        self.tokens.back().copied()
    }

    /// Return whether the cache contains one parser-relative offset.
    #[inline(always)]
    fn has(&self, offset: usize) -> bool {
        offset <= self.tokens.len()
    }

    /// Return whether no future token is cached.
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Save the lexer state after the current parser token.
    #[inline(always)]
    fn save_after_current(&mut self, lexer_state: LexerState, side_tokens_len: usize) {
        if self.after_current.is_none() {
            self.after_current = Some(LookaheadState {
                lexer: lexer_state,
                side_tokens_len,
            });
        }
    }

    /// Return the saved cursor state after the current parser token.
    #[inline(always)]
    fn state_after_current(&self) -> Option<LookaheadState> {
        self.after_current
    }

    /// Save the lexer boundary after the final cached token.
    #[inline(always)]
    fn push_boundary(&mut self, lexer_state: LexerState, side_tokens_len: usize) {
        debug_assert!(!self.tokens.is_empty());
        self.boundaries.push_back(LookaheadState {
            lexer: lexer_state,
            side_tokens_len,
        });
    }

    /// Push one future token.
    #[inline(always)]
    fn push(&mut self, token: Token) {
        self.tokens.push_back(token);
    }

    /// Pop the next future token into the parser cursor.
    ///
    /// The cache must contain at least one token.
    #[inline(never)]
    fn pop(&mut self) -> Token {
        debug_assert!(!self.tokens.is_empty());

        // SAFETY: callers check that the cache contains a token
        let token = unsafe { self.tokens.pop_front().unwrap_unchecked() };

        if self.tokens.is_empty() {
            debug_assert!(self.boundaries.is_empty());
            self.after_current = None;
        } else {
            self.after_current = self.boundaries.pop_front();
            debug_assert!(self.after_current.is_some());
        }

        token
    }
}

/// A streaming parser for one source file.
pub struct Parser {
    /// The source we're parsing.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The live lexer cursor.
    lexer: Lexer,
    /// The cached future tokens.
    lookahead: Lookahead,
    /// The tokens consumed by parser context-sensitive interpretation.
    tokens: Vec<Token>,
    /// The side token stream.
    side_tokens: Vec<Token>,
    /// The structured comments collected during lexing.
    comments: Vec<Comment>,
    /// The parser trivia retention mode.
    trivia_mode: ParserTriviaMode,
    /// The lexing mode for the next token read.
    contextual_lex_mode: ContextualLexMode,

    /// The current visible token at the parser cursor.
    current_token: Token,
    /// The previous semantic token end.
    previous_token_end: u32,
    /// The last consumed visible token.
    last_consumed_token: Token,
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
    /// The strings interned by this parser.
    pub strings: LocalStringPool,
    /// The shared string pool receiving parsed strings.
    shared_strings: Arc<StringPool>,

    /// Whether the source is an ambient declaration file.
    pub(crate) is_ambient: bool,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParserError>,
    /// The parser errors already reported for deduplication.
    reported_errors: HashSet<ParserError>,
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Return the source span for one parser token.
    #[inline(always)]
    fn token_source_span(&self, token: Token) -> Span {
        Span::new(self.file_id, token.start(), token.end())
    }

    /// Return one parser token as a full token span.
    #[inline(always)]
    fn token_span(&self, token: Token) -> TokenSpan {
        TokenSpan::new(token, self.file_id)
    }

    /// Return true when the current token starts after a line break.
    #[inline]
    pub(crate) fn current_token_is_on_new_line(&self) -> bool {
        self.current_token.is_on_new_line()
    }

    /// Return true when source trivia before the offset token contains a line break.
    #[inline]
    pub(crate) fn token_at_offset_is_on_new_line(&mut self, offset: usize) -> bool {
        self.token_at_offset(offset).is_on_new_line()
    }

    /// Return true when comments appear between the previous token and current token.
    pub(crate) fn current_token_has_leading_comment(&mut self) -> bool {
        self.source_range_has_comment(self.previous_token_end, self.current_token.start())
    }

    /// Return true when comments appear before one peeked token.
    pub(crate) fn token_has_leading_comment_after(
        &mut self,
        previous_end: u32,
        token: TokenSpan,
    ) -> bool {
        self.source_range_has_comment(previous_end, token.span.start)
    }

    /// Return true when one source range contains a line or block comment.
    fn source_range_has_comment(&mut self, start: u32, end: u32) -> bool {
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
        // bound malformed recursive input
        if self.recursive_descent_depth >= MAX_RECURSIVE_DESCENT_DEPTH {
            return Err(ParserError::unexpected_for(self.peek(), owner));
        }

        // periodically allow nested calls to grow onto another stack
        self.recursive_descent_depth += 1;
        let is_stack_check = self.recursive_descent_depth % STACK_CHECK_INTERVAL == 0;
        let result = if is_stack_check {
            ensure_sufficient_stack(|| parse(self))
        } else {
            parse(self)
        };

        // restore the caller depth
        self.recursive_descent_depth -= 1;

        result
    }

    /// Create one parser that appends into an existing DIR tree.
    fn new(
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

        // create the live lexer cursor
        let mut lexer = Lexer::new(file.clone());
        lexer.set_trivia_mode(options.trivia_mode);

        // initialize source-local parser state
        let file_id = file.id;
        let mut parser = Self {
            file,
            file_id,
            lexer,
            lookahead: Lookahead::new(),
            tokens: Vec::with_capacity(estimated_tokens),
            side_tokens: Vec::with_capacity(estimated_side_tokens),
            comments: Vec::with_capacity(estimated_comments),
            trivia_mode: options.trivia_mode,
            contextual_lex_mode: ContextualLexMode::Normal,
            current_token: Token::eof(0),
            previous_token_end: 0,
            last_consumed_token: Token::eof(0),
            is_finished: false,
            flags: ParserFlags::default(),
            preserve_parenthesized_wrappers: options.preserve_parenthesized_wrappers,
            recursive_descent_depth: 0,
            is_ambient: language.is_declaration(),
            tree,
            strings: LocalStringPool::new(),
            shared_strings: strings,
            errors: Vec::with_capacity(4),
            reported_errors: HashSet::with_capacity(4),
        };
        parser.read_first_token();

        parser
    }

    /// Create a parser for one source file.
    pub fn lex_file(file: Arc<File>, language: LanguageType, strings: Arc<StringPool>) -> Self {
        let options = ParserOptions {
            trivia_mode: ParserTriviaMode::Documentation,
            ..ParserOptions::default()
        };

        Self::lex_file_with_options(file, language, options, strings)
    }

    /// Create a parser for one module source file.
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

    /// Create a parser for one source file with explicit options.
    pub fn lex_file_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let module_id = ModuleId::new(PackageId::new(0), file.id.0);

        Self::lex_module_with_options(module_id, file, language, options, strings)
    }

    /// Create a parser for one module source file with explicit options.
    pub fn lex_module_with_options(
        module_id: ModuleId,
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let source_len = file.text().len();
        let estimated_tokens = source_len / ESTIMATED_TOKEN_BYTES;
        let tree = Tree::with_capacity(module_id, estimated_tokens);

        Self::lex_module_tree_with_options(file, language, options, strings, tree)
    }

    /// Create a parser that appends one module source file to an existing DIR tree.
    pub fn lex_module_tree_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
        tree: Tree,
    ) -> Self {
        Self::new(file, language, strings, tree, options)
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

    /// Publish locally interned strings and return the shared pool.
    pub fn publish_strings(&self) -> &Arc<StringPool> {
        self.shared_strings.extend(&self.strings);

        &self.shared_strings
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

    /// Return the current semantic tokens for parser tests.
    #[cfg(test)]
    #[inline]
    pub(crate) fn tokens(&self) -> Vec<TokenSpan> {
        self.tokens
            .iter()
            .copied()
            .map(|token| TokenSpan::new(token, self.file_id))
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

    /// Eat a tree opening `<`.
    #[inline]
    pub(crate) fn eat_tree_opening_angle(&mut self) -> ParserResult<()> {
        if !self.peek_is(TokenType::LessThan) {
            return Err(ParserError::expected(self.peek().span, TokenType::LessThan));
        }

        self.bump_tree_opening_angle();
        Ok(())
    }

    /// Advance past a tree opening `<`.
    #[inline]
    fn bump_tree_opening_angle(&mut self) {
        self.truncate_unread_tokens();
        self.contextual_lex_mode = ContextualLexMode::TreeTag;
        self.bump();
    }

    /// Enable or disable tree attribute value lexing for the next token.
    #[inline]
    pub(crate) fn set_tree_attribute_value(&mut self, enabled: bool) {
        self.truncate_unread_tokens();
        self.contextual_lex_mode = if enabled {
            ContextualLexMode::TreeAttributeValue
        } else {
            ContextualLexMode::Normal
        };
    }

    /// Read the next token in tree tag mode.
    #[inline]
    pub(crate) fn set_tree_tag_follow(&mut self) {
        self.truncate_unread_tokens();
        self.contextual_lex_mode = ContextualLexMode::TreeTag;
    }

    /// Bump the current token and read the next one in a contextual lexer mode.
    #[inline]
    pub(crate) fn bump_with_contextual_lex_mode(&mut self, mode: ContextualLexMode) {
        self.truncate_unread_tokens();
        self.contextual_lex_mode = mode;
        self.drop_side_tokens_covered_by_current();
        self.bump();
    }

    /// Re-lex the current token as a generic `<`.
    #[inline]
    pub(crate) fn re_lex_generic_l_angle(&mut self) -> bool {
        let token_type = self.current_token.ty();
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

    /// Eat one reference prefix operator.
    #[inline]
    pub(crate) fn eat_reference_prefix_operator(&mut self) -> ParserResult<TokenSpan> {
        if !self.re_lex_reference_prefix_operator() {
            return Err(ParserError::unexpected(self.peek()));
        }

        let token = self.peek();
        self.bump();

        Ok(token)
    }

    /// Re-lex the current token as one reference prefix operator.
    #[inline]
    fn re_lex_reference_prefix_operator(&mut self) -> bool {
        let token_type = self.current_token.ty();
        if matches!(
            token_type,
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor
        ) {
            return true;
        }

        if token_type != TokenType::LogicalAnd {
            return false;
        }

        let token = self.split_current_token_prefix(TokenType::ElementwiseAnd, 1);
        self.current_token = token;

        true
    }

    /// Eat one angle-close token with optional contextual follow mode.
    #[inline]
    fn eat_r_angle_close_with_mode(
        &mut self,
        follow_mode: Option<ContextualLexMode>,
    ) -> ParserResult<()> {
        let token_type = self.current_token.ty();
        if token_type == TokenType::GreaterThan {
            self.bump_after_r_angle_close(follow_mode);

            return Ok(());
        }

        if !matches!(
            token_type,
            TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            return Err(ParserError::unexpected(self.peek()));
        }

        let token = self.split_current_token_prefix(TokenType::GreaterThan, 1);
        self.current_token = token;

        self.bump_after_r_angle_close(follow_mode);

        Ok(())
    }

    /// Advance after consuming one angle-close token.
    #[inline]
    fn bump_after_r_angle_close(&mut self, follow_mode: Option<ContextualLexMode>) {
        if let Some(follow_mode) = follow_mode {
            self.bump_with_contextual_lex_mode(follow_mode);
        } else {
            self.bump();
        }
    }

    /// Re-lex the current `/` or `/=` token as a regex literal
    #[inline]
    pub(crate) fn re_lex_regex(&mut self) -> bool {
        let token_type = self.current_token.ty();
        if token_type == TokenType::Literal
            && matches!(
                self.current_token.literal(),
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
    fn split_current_token_prefix(&mut self, token_type: TokenType, prefix_len: u32) -> Token {
        self.truncate_unread_tokens();
        let current = self.current_token;
        debug_assert!(prefix_len > 0 && prefix_len <= current.len());

        let prefix = Token::simple(token_type, current.start(), prefix_len)
            .with_on_new_line(current.is_on_new_line());

        if current.len() > prefix_len {
            let rest_start = current.start() + prefix_len;
            self.lexer.set_position(rest_start as usize);
        }

        prefix
    }

    /// Reclassify the current slash token as one regex literal token.
    fn reclassify_current_divide_as_regex(&mut self) -> Token {
        self.truncate_unread_tokens();
        let current = self.current_token;
        let source = self.file.text();
        let mut index = current.start() as usize + 1;
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

        Token::new(
            TokenType::Literal,
            current.start(),
            end - current.start(),
            Some(TokenLiteral::RegexString { has_flags }),
        )
        .with_on_new_line(current.is_on_new_line())
    }

    /// Eat one typed angle-close token.
    #[inline]
    pub(crate) fn eat_type_angle_close(&mut self) -> ParserResult<()> {
        self.eat_r_angle_close_with_mode(None)
    }

    /// Eat one expression-position typed angle-close token.
    #[inline]
    pub(crate) fn eat_expression_type_angle_close(&mut self) -> ParserResult<()> {
        if !Self::starts_expression_type_angle_close(self.peek_token_type()) {
            return Err(ParserError::unexpected(self.peek()));
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

    /// Return whether the current token starts a tree literal closing tag.
    #[inline]
    pub(crate) fn peek_starts_tree_literal_close(&mut self) -> bool {
        self.peek_is(TokenType::LessThan) && self.next_token_type() == TokenType::Divide
    }

    /// Return owned token buffers after lexing to EOF.
    pub fn take_tokens(&mut self) -> (Vec<Token>, Vec<Token>) {
        self.truncate_unread_tokens();
        let mut tokens = mem::take(&mut self.tokens);
        tokens.push(self.current_token);

        if self.current_token.is(TokenType::End) {
            self.drain_lexer_side_tokens();

            return (tokens, mem::take(&mut self.side_tokens));
        }

        self.contextual_lex_mode = ContextualLexMode::Normal;
        loop {
            let token = self.read_token_from_lexer();
            let is_end = token.is(TokenType::End);
            tokens.push(token);

            if is_end {
                break;
            }
        }
        self.drain_lexer_side_tokens();

        (tokens, mem::take(&mut self.side_tokens))
    }

    /// Return owned full token span buffers after lexing to EOF.
    pub fn take_token_spans(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        let file_id = self.file_id;
        let (tokens, side_tokens) = self.take_tokens();
        let tokens = tokens
            .into_iter()
            .map(|token| TokenSpan::new(token, file_id))
            .collect();
        let side_tokens = side_tokens
            .into_iter()
            .map(|token| TokenSpan::new(token, file_id))
            .collect();

        (tokens, side_tokens)
    }

    /// Return the EOF span without forcing a full lex.
    #[inline]
    pub(crate) fn eof_span(&self) -> Span {
        Span::new(self.file_id, self.file.len, self.file.len)
    }

    /// Drop unread tokens and restore the lexer after the current token.
    fn truncate_unread_tokens(&mut self) {
        if let Some(state) = self.lookahead.state_after_current() {
            self.lexer.restore_state(state.lexer);
            self.side_tokens.truncate(state.side_tokens_len);
        }

        self.lookahead.clear();
    }

    /// Ensure the lookahead cache contains one parser-relative offset.
    fn ensure_lookahead(&mut self, offset: usize) {
        if offset == 0 || self.current_token.is(TokenType::End) {
            return;
        }

        if self.lookahead.state_after_current().is_none() {
            let lexer_state = self.lexer.state();
            let side_tokens_len = self.side_tokens.len();
            self.lookahead
                .save_after_current(lexer_state, side_tokens_len);
        }

        while !self.lookahead.has(offset) {
            if self
                .lookahead
                .last()
                .is_some_and(|token| token.is(TokenType::End))
            {
                break;
            }

            // preserve the boundary before extending an existing token sequence
            if self.lookahead.last().is_some() {
                let lexer_state = self.lexer.state();
                let side_tokens_len = self.side_tokens.len();
                self.lookahead.push_boundary(lexer_state, side_tokens_len);
            }

            let token = self.read_token_from_lexer();
            self.lookahead.push(token);
        }
    }

    /// Return one visible token without moving the parser cursor.
    pub(crate) fn token_at_offset(&mut self, offset: usize) -> Token {
        if offset == 0 {
            return self.current_token;
        }

        if let Some(token) = self.lookahead.get(offset) {
            return token;
        }

        self.ensure_lookahead(offset);

        self.lookahead
            .get(offset)
            .or_else(|| self.lookahead.last())
            .unwrap_or(self.current_token)
    }

    /// Return one visible token type without moving the parser cursor.
    #[inline(always)]
    pub(crate) fn token_type_at_offset(&mut self, offset: usize) -> TokenType {
        if offset == 0 {
            return self.current_token.ty();
        }

        if let Some(token) = self.lookahead.get(offset) {
            return token.ty();
        }

        self.token_at_offset(offset).ty()
    }

    /// Return one visible token as a keyword without moving the parser cursor.
    #[inline(always)]
    pub(crate) fn keyword_at_offset(&mut self, offset: usize) -> Option<Keyword> {
        if offset == 0 {
            if !self.current_token.is(TokenType::Identifier) {
                return None;
            }

            if let Some(keyword) = self.current_token.classified_keyword() {
                return keyword;
            }

            return keyword_from_identifier(self.get_token_str(self.current_token));
        }

        if let Some(token) = self.lookahead.get(offset) {
            if !token.is(TokenType::Identifier) {
                return None;
            }

            if let Some(keyword) = token.classified_keyword() {
                return keyword;
            }

            return keyword_from_identifier(self.get_token_str(token));
        }

        let token = self.token_at_offset(offset);
        if !token.is(TokenType::Identifier) {
            return None;
        }

        if let Some(keyword) = token.classified_keyword() {
            return keyword;
        }

        keyword_from_identifier(self.get_token_str(token))
    }

    /// Read the first token into the parser cursor.
    #[inline]
    fn read_first_token(&mut self) {
        self.current_token = self.read_token_from_lexer();
    }

    /// Read the next token into the parser cursor.
    #[inline]
    fn read_next_token(&mut self) {
        if self.lookahead.is_empty() {
            self.current_token = self.read_token_from_lexer();

            return;
        }

        self.current_token = self.lookahead.pop();
    }

    /// Read one token from the live lexer in the current contextual mode.
    fn read_token_from_lexer(&mut self) -> Token {
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
        self.lexer.drain_side_tokens_into(&mut self.side_tokens);
    }

    /// Parse root expressions as an implicit namespace.
    fn parse_roots(&mut self, attach_comments: bool) -> Vec<LocalNodeId<Expression>> {
        let start = self.span_start();
        let expressions = self.with_token_recovery(
            &start,
            |parser| parser.eat_block_body(BlockForm::Implicit),
            Vec::new(),
            TokenType::End,
        );
        self.drain_lexer_trivia();

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

    /// Report one parser error unless an identical error was already reported.
    #[cold]
    #[inline(never)]
    pub(crate) fn report_error(&mut self, error: &ParserError) {
        if self.reported_errors.insert(*error) {
            self.errors.push(*error);
        }
    }

    /// Build source diagnostics from parser errors.
    pub fn diagnostics(&self) -> DiagnosticCollection {
        if self.errors.is_empty() {
            return DiagnosticCollection::new();
        }

        let content = self.file.content_id();
        let diagnostics = self
            .errors
            .iter()
            .map(|error| error.to_diagnostic(content, self.file_id))
            .collect();

        DiagnosticCollection::from_diagnostics(diagnostics)
    }

    /// Build one source diagnostic from one parser error.
    pub fn diagnostic(&self, error: &ParserError) -> Diagnostic {
        let content = self.file.content_id();

        error.to_diagnostic(content, self.file_id)
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
        let (lexer_state, side_tokens_len) =
            if let Some(state) = self.lookahead.state_after_current() {
                (state.lexer, state.side_tokens_len)
            } else {
                (self.lexer.state(), self.side_tokens.len())
            };
        ParserCursorCheckpoint {
            lexer_state,
            contextual_lex_mode: self.contextual_lex_mode,
            tokens_len: self.tokens.len(),
            side_tokens_len,
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
        self.lookahead.clear();
        self.lexer.restore_state(checkpoint.lexer_state);
        self.contextual_lex_mode = checkpoint.contextual_lex_mode;
        self.tokens.truncate(checkpoint.tokens_len);
        self.side_tokens.truncate(checkpoint.side_tokens_len);
        self.current_token = checkpoint.current_token;
        self.previous_token_end = checkpoint.previous_token_end;
        self.last_consumed_token = checkpoint.last_consumed_token;
    }

    /// Restore the parser and tree to one full checkpoint.
    pub fn restore(&mut self, checkpoint: ParserCheckpoint) {
        self.rewind(checkpoint.cursor);
        self.tree.restore_to_mark(checkpoint.tree_mark);
        self.restore_errors(checkpoint.error_count);
    }

    /// Restore parser errors to one checkpoint.
    fn restore_errors(&mut self, error_count: usize) {
        for error in self.errors.drain(error_count..) {
            self.reported_errors.remove(&error);
        }
    }

    /// Run a closure against a speculative parser cursor.
    pub(crate) fn lookahead<T>(&mut self, func: impl FnOnce(&mut Self) -> T) -> T {
        let checkpoint = self.cursor_checkpoint();
        let result = func(self);
        self.rewind(checkpoint);

        result
    }

    /// Return the next parser token without consuming it.
    #[inline]
    pub(crate) fn next_token(&mut self) -> Token {
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
        let start = start.current_token.start();
        let end = self.previous_token_end.max(start);
        Span::new(self.file_id, start, end)
    }

    /// Return the span between two parser span starts.
    #[inline(always)]
    pub fn get_span_between(&self, start: &ParserSpanStart, end: &ParserSpanStart) -> Span {
        Span::new(
            self.file_id,
            start.current_token.start(),
            end.current_token.end(),
        )
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &str {
        self.file.get_span_str(span).unwrap_or_default()
    }

    /// Intern the source text backing one span.
    #[inline]
    pub(crate) fn intern_span(&mut self, span: Span) -> StringId {
        let text = self.file.span_str(span);

        self.strings.intern(text)
    }

    /// Get the source text backing one file-local byte range.
    #[inline]
    pub fn get_range_str(&self, range: ByteRange) -> &str {
        let span = Span::new(self.file_id, range.start, range.end);

        self.get_span_str(span)
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_span_str(&self, token: TokenSpan) -> &str {
        self.file.get_span_str(token.span).unwrap_or_default()
    }

    /// Gets the str source backing a token.
    #[inline]
    pub fn get_token_str(&self, token: Token) -> &str {
        let span = self.token_source_span(token);

        self.file.get_span_str(span).unwrap_or_default()
    }

    /// Get the source text for the current token.
    #[inline]
    pub(crate) fn current_token_str(&self) -> &str {
        self.get_token_str(self.current_token)
    }

    /// Get the current token.
    #[inline]
    pub(crate) const fn current_token(&self) -> Token {
        self.current_token
    }

    /// Return the current token as a keyword.
    #[inline]
    pub(crate) fn current_keyword(&self) -> Option<Keyword> {
        if !self.current_token.is(TokenType::Identifier) {
            return None;
        }

        if let Some(keyword) = self.current_token.classified_keyword() {
            return keyword;
        }

        keyword_from_identifier(self.current_token_str())
    }

    /// Return true when the current identifier has the expected source text.
    #[inline]
    pub(crate) fn current_identifier_str_is(&self, expected: &str) -> bool {
        self.current_token.is(TokenType::Identifier) && self.current_token_str() == expected
    }

    /// Return true when the current identifier is `global`.
    #[inline]
    pub(crate) fn is_global_identifier(&self) -> bool {
        self.current_identifier_str_is("global")
    }

    /// Return true when the current identifier is `module`.
    #[inline]
    pub(crate) fn is_module_identifier(&self) -> bool {
        self.current_identifier_str_is("module")
    }

    /// Get the previous Token.
    #[inline]
    pub fn prev(&self) -> Option<TokenSpan> {
        (self.previous_token_end > 0).then_some(self.token_span(self.last_consumed_token))
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

    /// Return the current token.
    #[inline]
    pub fn peek(&self) -> TokenSpan {
        self.token_span(self.current_token)
    }

    /// Return the current token type.
    #[inline]
    pub fn peek_token_type(&self) -> TokenType {
        self.current_token.ty()
    }

    /// Return whether the current token matches the given type.
    #[inline]
    pub fn peek_is(&self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_is requires semantic token type"
        );

        self.current_token.is(token_type)
    }

    /// Return true when more tokens remain before End.
    #[inline]
    pub fn has_more_tokens(&self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Eat and return the current token.
    #[inline]
    pub fn eat(&mut self) -> TokenSpan {
        let token = self.token_span(self.current_token);
        self.bump();

        token
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.end();
        self.record_consumed_token(self.current_token);
        self.read_next_token();
    }

    /// Record one consumed semantic token.
    #[inline]
    fn record_consumed_token(&mut self, token: Token) {
        if !token.is(TokenType::End) {
            self.tokens.push(token);
        }
    }

    /// Drop trivia tokens that are now part of a virtual tree child token.
    fn drop_side_tokens_covered_by_current(&mut self) {
        let token = self.current_token;
        let is_tree_text = token.is(TokenType::Literal)
            && matches!(
                token.literal(),
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
            side_token.start() < token.start() || side_token.end() > token.end()
        });
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&self, token_type: TokenType) -> ParserResult<TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek();
        if next.token.is(token_type) {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_token_in(&self, token_types: &[TokenType]) -> ParserResult<TokenSpan> {
        let next = self.peek();
        if token_types.contains(&next.token.ty()) {
            Ok(next)
        } else {
            Err(ParserError::unexpected(next))
        }
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParserResult<TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "eat_token requires semantic token type"
        );
        let current = self.eat();
        if current.token.is(token_type) {
            Ok(current)
        } else {
            Err(ParserError::unexpected(current))
        }
    }

    /// Eat the current token when it has the requested type.
    #[inline]
    pub fn eat_token_if(&mut self, token_type: TokenType) -> bool {
        if self.peek_is(token_type) {
            self.bump();

            true
        } else {
            false
        }
    }

    /// Eat one tree tag close token and advance in the requested mode.
    #[inline]
    pub(crate) fn eat_tree_tag_close(
        &mut self,
        follow_mode: ContextualLexMode,
    ) -> ParserResult<()> {
        self.eat_r_angle_close_with_mode(Some(follow_mode))
    }

    /// Eat a token in a list of tokens.
    #[inline]
    pub fn eat_token_in(&mut self, token_types: &[TokenType]) -> ParserResult<TokenType> {
        let current = self.eat();
        if token_types.contains(&current.token.ty()) {
            Ok(current.token.ty())
        } else {
            Err(ParserError::unexpected(current))
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
    /// The lexer state at parser checkpoint time.
    lexer_state: LexerState,
    /// The contextual lexing mode at checkpoint time.
    contextual_lex_mode: ContextualLexMode,
    /// The consumed token count at checkpoint time.
    tokens_len: usize,
    /// The side token count at checkpoint time.
    side_tokens_len: usize,
    /// The parser owned current token at checkpoint time.
    current_token: Token,
    /// The previous semantic token end at checkpoint time.
    previous_token_end: u32,
    /// The last consumed visible token at checkpoint time.
    last_consumed_token: Token,
}

/// Lightweight parser position used for span construction.
#[derive(Debug, Copy, Clone)]
pub struct ParserSpanStart {
    /// The parser owned current token at span start time.
    current_token: Token,
}

impl ParserSpanStart {
    /// Return the token span that started this source span.
    #[inline]
    pub(crate) fn token_span(&self, file_id: FileId) -> Span {
        self.current_token.span(file_id)
    }

    /// Return the start of the token that started this span.
    #[inline]
    pub(crate) fn token_start(&self) -> u32 {
        self.current_token.start()
    }

    /// Return the end of the token that started this span.
    #[inline]
    pub(crate) fn token_end(&self) -> u32 {
        self.current_token.end()
    }
}
