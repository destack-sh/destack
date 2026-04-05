use std::str::FromStr;
use std::sync::Arc;

use destack_ast::{Keyword, TokenSpan, TokenType};
use destack_source::{File, LanguageType, Span};

use super::lex::is_semantic;
use super::lexer::{Lexer, LexerSnapshot, TreeState};

/// Return a keyword for an identifier when it can match keyword shape.
#[inline]
pub(crate) fn keyword_from_identifier(identifier: &str) -> Option<Keyword> {
    // quick reject using identifier length bounds for known keywords
    let bytes = identifier.as_bytes();
    let length = bytes.len();
    if !(2..=11).contains(&length) {
        return None;
    }

    // quick reject using keyword (length, first byte) pairs before full string matching
    let first = *bytes.first()?;
    let can_match_keyword = matches!(
        (length, first),
        (2, b'a' | b'd' | b'i' | b'o' | b't')
            | (3, b'a' | b'f' | b'g' | b'l' | b'n' | b's' | b't' | b'v')
            | (
                4,
                b'c' | b'e' | b'f' | b'g' | b'l' | b'm' | b'n' | b's' | b't' | b'v' | b'w'
            )
            | (
                5,
                b'a' | b'b' | b'c' | b'i' | b'k' | b'm' | b'n' | b's' | b't' | b'u' | b'w' | b'y'
            )
            | (6, b'a' | b'd' | b'e' | b'i' | b'p' | b'r' | b's' | b't')
            | (7, b'a' | b'd' | b'e' | b'f' | b'n' | b'p')
            | (8, b'a' | b'c' | b'd' | b'f' | b'o' | b'p' | b'r')
            | (9, b'e' | b'i' | b'n' | b'p' | b's')
            | (10, b'i')
            | (11, b'c')
    );
    if !can_match_keyword {
        return None;
    }

    Keyword::from_str(identifier).ok()
}

/// Dense metadata for one materialized semantic token.
#[derive(Debug, Copy, Clone)]
struct SemanticTokenData {
    /// The matching close token index for an opening delimiter.
    matching_pair: u32,
    /// The contextual keyword classification for identifier tokens.
    keyword: Option<Keyword>,
    /// Whether trivia before this token contains a line terminator.
    has_line_terminator_before: bool,
    /// Whether trivia before this token contains a comment token.
    has_comment_before: bool,
}

impl SemanticTokenData {
    /// Return an empty semantic token metadata record.
    #[inline]
    const fn new(keyword: Option<Keyword>, has_line_terminator_before: bool) -> Self {
        Self {
            matching_pair: u32::MAX,
            keyword,
            has_line_terminator_before,
            has_comment_before: false,
        }
    }
}

/// Dense retained trivia bounds for one semantic token.
#[derive(Debug, Copy, Clone)]
pub struct SideRange {
    /// The starting side token index.
    pub start: u32,
    /// The ending side token index.
    pub end: u32,
}

impl SideRange {
    /// Return one empty side range.
    #[inline]
    const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// Snapshot of token stream state for speculative parsing.
#[derive(Debug, Clone)]
pub struct TokenStreamMark {
    /// The lexer snapshot for restoring positions and stacks.
    pub(super) lexer: LexerSnapshot,
    /// The number of semantic tokens captured in the mark.
    pub(super) tokens_len: usize,
    /// The number of side tokens captured in the mark.
    pub(super) side_tokens_len: usize,
    /// The number of comment side token indexes captured in the mark.
    pub(super) comment_side_tokens_len: usize,
    /// The open parenthesis stack at mark time.
    pub(super) paren_stack: Vec<usize>,
    /// The open brace stack at mark time.
    pub(super) brace_stack: Vec<usize>,
    /// The open bracket stack at mark time.
    pub(super) bracket_stack: Vec<usize>,
    /// Side trivia ranges per semantic token from the mark tail.
    pub(super) leading_side_ranges_tail: Vec<SideRange>,
    /// The pending side trivia start index for the next semantic token.
    pub(super) pending_leading_side_start: usize,
    /// Whether side trivia since the last semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// Whether side trivia since the last semantic token had a comment token.
    pub(super) pending_comment_before_next: bool,
    /// Whether this stream snapshot has comment style side annotations.
    pub(super) has_comment_side_tokens: bool,
    /// Whether this stream snapshot has semantic newline tokens.
    pub(super) has_semantic_newline_tokens: bool,
    /// Whether this stream snapshot has non-newline semantic tokens.
    pub(super) has_attachable_semantic_tokens: bool,
    /// Pending synthetic token from operator decomposition.
    pub(super) split_token: Option<TokenSpan>,
    /// Whether the pending split token has already been consumed.
    pub(super) split_token_consumed: bool,
}

/// Lazy token stream that drives the lexer on demand.
#[derive(Debug)]
pub struct TokenStream {
    /// The underlying lexer.
    lexer: Lexer,
    /// Whether side trivia buffers should be retained.
    retain_trivia_tokens: bool,
    /// The semantic tokens produced so far.
    tokens: Vec<TokenSpan>,
    /// The side tokens produced so far.
    side_tokens: Vec<TokenSpan>,
    /// Indexes of comment-like side tokens in `side_tokens`.
    comment_side_token_indexes: Vec<u32>,
    /// Dense metadata for semantic token indexes.
    token_data: Vec<SemanticTokenData>,
    /// Side trivia ranges before each semantic token.
    leading_side_ranges_by_token: Vec<SideRange>,
    /// Side trivia start index for the next semantic token.
    pending_leading_side_start: usize,
    /// The stack of open parenthesis token indexes.
    paren_stack: Vec<usize>,
    /// The stack of open brace token indexes.
    brace_stack: Vec<usize>,
    /// The stack of open bracket token indexes.
    bracket_stack: Vec<usize>,
    /// Whether side trivia since the previous semantic token had a line terminator.
    pending_line_terminator_before_next: bool,
    /// Whether side trivia since the previous semantic token had a comment token.
    pending_comment_before_next: bool,
    /// Whether any comment style side trivia token was seen.
    has_comment_side_tokens: bool,
    /// Whether any semantic newline token was seen.
    has_semantic_newline_tokens: bool,
    /// Whether any non-newline semantic token was seen.
    has_attachable_semantic_tokens: bool,
    /// Whether EOF has been reached.
    is_finished: bool,
    /// The cached EOF token, when available.
    eof_token: Option<TokenSpan>,
    /// Pending synthetic token from parser level operator decomposition.
    split_token: Option<TokenSpan>,
    /// Whether the split token has been consumed.
    split_token_consumed: bool,
}

impl TokenStream {
    /// Create a new token stream for a file and language.
    pub fn new(file: Arc<File>, language: LanguageType) -> Self {
        // estimate token counts from file length
        let source_len = file.text().len();
        let estimated_tokens = source_len / 6;
        let semantic_token_capacity = estimated_tokens;
        let side_token_capacity = estimated_tokens / 2;

        // allocate buffers using the same heuristic as the lexer
        let tokens = Vec::with_capacity(semantic_token_capacity);
        let side_tokens = Vec::with_capacity(side_token_capacity);
        // build the stream
        Self {
            lexer: Lexer::new(file, language),
            retain_trivia_tokens: true,
            tokens,
            side_tokens,
            comment_side_token_indexes: Vec::with_capacity(estimated_tokens / 24),
            token_data: Vec::with_capacity(semantic_token_capacity),
            leading_side_ranges_by_token: Vec::with_capacity(semantic_token_capacity),
            pending_leading_side_start: 0,
            paren_stack: Vec::with_capacity(semantic_token_capacity / 64),
            brace_stack: Vec::with_capacity(semantic_token_capacity / 64),
            bracket_stack: Vec::with_capacity(semantic_token_capacity / 64),
            pending_line_terminator_before_next: false,
            pending_comment_before_next: false,
            has_comment_side_tokens: false,
            has_semantic_newline_tokens: false,
            has_attachable_semantic_tokens: false,
            is_finished: false,
            eof_token: None,
            split_token: None,
            split_token_consumed: false,
        }
    }

    /// Return the current semantic tokens.
    #[inline]
    pub fn tokens(&self) -> &[TokenSpan] {
        &self.tokens
    }

    /// Return whether side trivia buffers are retained.
    #[inline]
    pub fn retains_trivia_tokens(&self) -> bool {
        self.retain_trivia_tokens
    }

    /// Enable or disable side trivia retention before lexing begins.
    #[inline]
    pub fn set_retain_trivia_tokens(&mut self, retain_trivia_tokens: bool) {
        debug_assert!(
            self.tokens.is_empty() && self.side_tokens.is_empty(),
            "trivia retention must be configured before lexing starts"
        );
        self.retain_trivia_tokens = retain_trivia_tokens;
    }

    /// Return the current side tokens.
    #[inline]
    pub fn side_tokens(&self) -> &[TokenSpan] {
        &self.side_tokens
    }

    /// Return side token indexes for comment-like trivia tokens.
    #[inline]
    pub fn comment_side_token_indexes(&self) -> &[u32] {
        &self.comment_side_token_indexes
    }

    /// Return the leading side trivia range for a semantic token index.
    #[inline]
    pub fn leading_side_range(&mut self, index: usize) -> (usize, usize) {
        self.ensure_token(index);
        self.leading_side_range_materialized(index)
    }

    /// Return the leading side trivia range for a semantic token index.
    #[inline]
    fn leading_side_range_materialized(&self, index: usize) -> (usize, usize) {
        if index >= self.leading_side_ranges_by_token.len() {
            let side_len = self.side_tokens.len();
            return (side_len, side_len);
        }

        let range = self.leading_side_ranges_by_token[index];
        let start = range.start as usize;
        let end = range.end as usize;
        (start, end)
    }

    /// Return leading side trivia ranges for semantic tokens.
    #[inline]
    pub fn leading_side_ranges(&self) -> &[SideRange] {
        &self.leading_side_ranges_by_token
    }

    /// Return true once EOF has been reached.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Return true when a split token is present and unconsumed.
    #[inline]
    pub fn has_active_split(&self) -> bool {
        self.split_token.is_some() && !self.split_token_consumed
    }

    /// Return true when split token state exists.
    #[inline]
    pub fn has_split_state(&self) -> bool {
        self.split_token.is_some()
    }

    /// Return the active split token.
    #[inline]
    pub fn active_split_token(&self) -> Option<&TokenSpan> {
        if self.has_active_split() {
            self.split_token.as_ref()
        } else {
            None
        }
    }

    /// Mark the active split token as consumed.
    #[inline]
    pub fn mark_split_token_consumed(&mut self) -> bool {
        if !self.has_active_split() {
            return false;
        }

        self.split_token_consumed = true;
        true
    }

    /// Return the split token regardless of consumed state.
    #[inline]
    pub fn split_token_ref(&self) -> Option<&TokenSpan> {
        self.split_token.as_ref()
    }

    /// Set a split token and mark it as unconsumed.
    #[inline]
    pub fn set_split_token(&mut self, token: TokenSpan) {
        self.split_token = Some(token);
        self.split_token_consumed = false;
    }

    /// Clear split token state.
    #[inline]
    pub fn clear_split_token(&mut self) {
        self.split_token = None;
        self.split_token_consumed = false;
    }

    /// Return true when the active split token matches the given type.
    #[inline]
    pub fn has_split_token(&self, token_type: TokenType) -> bool {
        self.split_token
            .map(|token| token.token.ty == token_type && !self.split_token_consumed)
            .unwrap_or(false)
    }

    /// Allow or disallow tree literal lexing.
    #[inline]
    pub fn set_allow_tree_literals(&mut self, allow: bool) {
        self.lexer.options.allow_tree_literals = allow;
    }

    /// Return true when the lexer is currently inside a tree literal.
    #[inline]
    pub fn in_tree_literal(&self) -> bool {
        self.lexer.tree_state() != TreeState::None
    }

    /// Return true when the lexer is inside any tree expression container.
    #[inline]
    pub fn in_tree_expression_container(&self) -> bool {
        self.lexer.in_tree_expression_container()
    }

    /// Return true when tree literal lexing is enabled.
    #[inline]
    pub fn allow_tree_literals(&self) -> bool {
        self.lexer.options.allow_tree_literals
    }

    /// Enter tree literal opening tag mode.
    pub fn enter_tree_opening_tag(&mut self) {
        self.lexer.push_tree_state(TreeState::OpeningTag);
    }

    /// Snapshot token stream state for speculative parsing.
    #[inline]
    pub fn mark(&self) -> TokenStreamMark {
        TokenStreamMark {
            lexer: self.lexer.snapshot(),
            tokens_len: self.tokens.len(),
            side_tokens_len: self.side_tokens.len(),
            comment_side_tokens_len: self.comment_side_token_indexes.len(),
            paren_stack: self.paren_stack.clone(),
            brace_stack: self.brace_stack.clone(),
            bracket_stack: self.bracket_stack.clone(),
            leading_side_ranges_tail: self.leading_side_ranges_by_token.to_vec(),
            pending_leading_side_start: self.pending_leading_side_start,
            pending_line_terminator_before_next: self.pending_line_terminator_before_next,
            pending_comment_before_next: self.pending_comment_before_next,
            has_comment_side_tokens: self.has_comment_side_tokens,
            has_semantic_newline_tokens: self.has_semantic_newline_tokens,
            has_attachable_semantic_tokens: self.has_attachable_semantic_tokens,
            split_token: self.split_token,
            split_token_consumed: self.split_token_consumed,
        }
    }

    /// Restore token stream state from a snapshot.
    pub fn restore(&mut self, mark: TokenStreamMark) {
        let TokenStreamMark {
            lexer,
            tokens_len,
            side_tokens_len,
            comment_side_tokens_len,
            paren_stack,
            brace_stack,
            bracket_stack,
            leading_side_ranges_tail,
            pending_leading_side_start,
            pending_line_terminator_before_next,
            pending_comment_before_next,
            has_comment_side_tokens,
            has_semantic_newline_tokens,
            has_attachable_semantic_tokens,
            split_token,
            split_token_consumed,
        } = mark;

        // restore lexer state and truncate token buffers
        let semantic_tokens_changed = self.tokens.len() != tokens_len;
        let side_tokens_changed = self.side_tokens.len() != side_tokens_len;
        self.lexer.restore(lexer);
        if side_tokens_changed {
            self.side_tokens.truncate(side_tokens_len);
        }
        self.comment_side_token_indexes
            .truncate(comment_side_tokens_len);
        self.pending_line_terminator_before_next = pending_line_terminator_before_next;
        self.pending_comment_before_next = pending_comment_before_next;
        self.pending_leading_side_start = pending_leading_side_start;
        self.has_comment_side_tokens = has_comment_side_tokens;
        self.has_semantic_newline_tokens = has_semantic_newline_tokens;
        self.has_attachable_semantic_tokens = has_attachable_semantic_tokens;
        self.split_token = split_token;
        self.split_token_consumed = split_token_consumed;

        // fast path: no semantic token changes, caches are still valid
        if !semantic_tokens_changed {
            return;
        }

        self.tokens.truncate(tokens_len);
        self.token_data.truncate(tokens_len);
        self.leading_side_ranges_by_token.truncate(tokens_len);

        // restore leading side data
        for (offset, side_range) in leading_side_ranges_tail.into_iter().enumerate() {
            self.leading_side_ranges_by_token[offset] = side_range;
        }

        // restore matching pair cache and open delimiter stacks
        for open_index in paren_stack
            .iter()
            .copied()
            .chain(brace_stack.iter().copied())
            .chain(bracket_stack.iter().copied())
        {
            self.token_data[open_index].matching_pair = u32::MAX;
        }
        self.paren_stack = paren_stack;
        self.brace_stack = brace_stack;
        self.bracket_stack = bracket_stack;

        // reset EOF tracking to the restored tail
        self.eof_token = self
            .tokens
            .last()
            .copied()
            .filter(|token| token.token.ty == TokenType::End);
        self.is_finished = self.eof_token.is_some();
    }

    /// Ensure a token exists at the given index.
    pub fn ensure_token(&mut self, index: usize) {
        // hot fast path: token is already available
        if index < self.tokens.len() {
            return;
        }

        // hot fast path: EOF already reached
        if self.is_finished {
            return;
        }

        while !self.is_finished && self.tokens.len() <= index {
            self.lex_next();
        }
    }

    /// Get the token at a given index, lexing as needed.
    pub fn token(&mut self, index: usize) -> Option<TokenSpan> {
        self.ensure_token(index);
        self.tokens.get(index).copied()
    }

    /// Get the EOF token, lexing until the end if needed.
    pub fn eof_token(&mut self) -> TokenSpan {
        // lex until EOF is reached
        while !self.is_finished {
            self.lex_next();
        }

        // return the cached EOF token or synthesize a fallback
        if let Some(token) = self.eof_token {
            return token;
        }

        TokenSpan {
            span: Span {
                file: self.lexer.file_id,
                start: 0,
                end: 0,
            },
            token: destack_ast::Token::end(),
        }
    }

    /// Ensure all tokens are lexed.
    pub fn lex_to_end(&mut self) {
        // fast path: one-pass full-file lex for non tree literal sources
        if !self.is_finished
            && !self.allow_tree_literals()
            && self.tokens.is_empty()
            && self.side_tokens.is_empty()
        {
            self.lex_to_end_non_tree();
            return;
        }

        while !self.is_finished {
            self.lex_next();
        }
    }

    /// Return owned token buffers and leave the stream empty.
    pub fn take_tokens(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        // ensure all tokens are available
        self.lex_to_end();

        // drain token buffers
        let tokens = std::mem::take(&mut self.tokens);
        let side_tokens = std::mem::take(&mut self.side_tokens);
        self.comment_side_token_indexes.clear();

        // reset caches and stacks for any follow-up access
        self.token_data.clear();
        self.leading_side_ranges_by_token.clear();
        self.pending_leading_side_start = 0;
        self.paren_stack.clear();
        self.brace_stack.clear();
        self.bracket_stack.clear();
        self.pending_line_terminator_before_next = false;
        self.pending_comment_before_next = false;
        self.has_comment_side_tokens = false;
        self.has_semantic_newline_tokens = false;
        self.has_attachable_semantic_tokens = false;
        self.clear_split_token();

        (tokens, side_tokens)
    }

    /// Return the materialized line terminator flag for a semantic token index.
    #[inline]
    pub(crate) fn materialized_line_terminator_before(&self, index: usize) -> bool {
        self.token_data
            .get(index)
            .map(|data| data.has_line_terminator_before)
            .unwrap_or(false)
    }

    /// Return the cached keyword for a materialized semantic token index.
    #[inline]
    pub(crate) fn materialized_keyword(&self, index: usize) -> Option<Keyword> {
        self.token_data.get(index).and_then(|data| data.keyword)
    }

    /// Return whether trivia before a semantic token index had a comment token.
    #[inline]
    pub fn comment_before(&mut self, index: usize) -> bool {
        if !self.retain_trivia_tokens {
            return false;
        }

        // hot fast path: full token stream is already materialized
        if self.is_finished {
            return self
                .token_data
                .get(index)
                .map(|data| data.has_comment_before)
                .unwrap_or(false);
        }

        self.ensure_token(index);
        self.token_data
            .get(index)
            .map(|data| data.has_comment_before)
            .unwrap_or(false)
    }

    /// Return whether an identifier token contains escape syntax.
    #[inline]
    pub fn identifier_has_escape(&mut self, index: usize) -> bool {
        self.ensure_token(index);

        let Some(token) = self.tokens.get(index) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }

        self.lexer
            .get_span_str(token.span)
            .as_bytes()
            .contains(&b'\\')
    }

    /// Return true when lexing reached EOF.
    #[inline]
    pub fn is_lexed_to_end(&self) -> bool {
        self.is_finished
    }

    /// Return the matching close token index for an opening token, if known.
    pub fn matching_pair(&mut self, index: usize) -> Option<usize> {
        // hot fast path: full token stream is already materialized
        if self.is_finished {
            let value = self
                .token_data
                .get(index)
                .map(|data| data.matching_pair)
                .unwrap_or(u32::MAX);
            if value == u32::MAX {
                return None;
            }

            return Some(value as usize);
        }

        self.ensure_token(index);

        let value = self
            .token_data
            .get(index)
            .map(|data| data.matching_pair)
            .unwrap_or(u32::MAX);
        if value == u32::MAX {
            return None;
        }

        Some(value as usize)
    }

    /// Lex the next token from the underlying lexer.
    fn lex_next(&mut self) {
        // stop once EOF has already been reached
        if self.is_finished {
            return;
        }

        self.lex_one();
    }

    /// Lex all tokens in a single pass for non tree literal sources.
    fn lex_to_end_non_tree(&mut self) {
        while !self.is_finished {
            self.lex_one();
        }
    }

    /// Lex one token and route it through the shared stream update path.
    #[inline]
    fn lex_one(&mut self) {
        let start = self.lexer.pos as u32;
        let token = self.lexer.advance();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.lexer.file_id,
                start,
                end: start + token.len,
            },
        };

        if is_semantic(token.ty) {
            self.push_semantic_token(token_span);
        } else {
            let has_line_terminator = self.lexer.side_token_had_line_terminator();
            self.push_side_token(token_span, has_line_terminator);
        }

        if token.ty == TokenType::End {
            self.is_finished = true;
            self.eof_token = Some(token_span);
        }
    }

    /// Push a side token and update stream flags that depend on side tokens.
    #[inline]
    fn push_side_token(&mut self, token_span: TokenSpan, has_line_terminator: bool) {
        let is_comment = matches!(
            token_span.token.ty,
            TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        );
        if self.retain_trivia_tokens && is_comment {
            self.has_comment_side_tokens = true;
            self.pending_comment_before_next = true;
            let side_index = self.side_tokens.len() as u32;
            self.comment_side_token_indexes.push(side_index);
        }

        if self.retain_trivia_tokens {
            self.side_tokens.push(token_span);
        }
        if has_line_terminator {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push a semantic token and update indexes.
    fn push_semantic_token(&mut self, token_span: TokenSpan) {
        let token_index = self.tokens.len();
        let has_line_terminator_before = self.pending_line_terminator_before_next;
        let keyword = if token_span.token.ty == TokenType::Identifier {
            keyword_from_identifier(self.lexer.get_span_str(token_span.span))
        } else {
            None
        };
        // add token and dense metadata
        self.tokens.push(token_span);
        self.token_data
            .push(SemanticTokenData::new(keyword, has_line_terminator_before));
        if self.retain_trivia_tokens {
            self.token_data[token_index].has_comment_before = self.pending_comment_before_next;
            let leading_side_start = self.pending_leading_side_start as u32;
            let leading_side_end = self.side_tokens.len() as u32;
            self.leading_side_ranges_by_token
                .push(SideRange::new(leading_side_start, leading_side_end));
            self.pending_leading_side_start = self.side_tokens.len();
        }
        self.pending_line_terminator_before_next = token_span.token.ty == TokenType::Newline;
        self.pending_comment_before_next = false;
        if self.retain_trivia_tokens {
            if token_span.token.ty == TokenType::Newline {
                self.has_semantic_newline_tokens = true;
            } else if token_span.token.ty != TokenType::End {
                self.has_attachable_semantic_tokens = true;
            }
        }

        // update lexer context for regex and tree rules
        self.lexer.track_semantic_token(token_span);

        // update matching pairs for brackets
        match token_span.token.ty {
            TokenType::OpenParenthesis => self.paren_stack.push(token_index),
            TokenType::CloseParenthesis => {
                if let Some(open) = self.paren_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                }
            }
            TokenType::OpenBrace => self.brace_stack.push(token_index),
            TokenType::CloseBrace => {
                if let Some(open) = self.brace_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                }
            }
            TokenType::OpenBracket => self.bracket_stack.push(token_index),
            TokenType::CloseBracket => {
                if let Some(open) = self.bracket_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                }
            }
            _ => {}
        }
    }

    /// Return true when comment style side annotation tokens were seen.
    #[inline]
    pub fn has_comment_tokens(&self) -> bool {
        self.has_comment_side_tokens
    }

    /// Return true when semantic newline tokens were seen.
    #[inline]
    pub fn has_blank_line_tokens(&self) -> bool {
        self.has_semantic_newline_tokens
    }

    /// Return true when non-newline semantic tokens were seen.
    #[inline]
    pub fn has_attachable_semantic_tokens(&self) -> bool {
        self.has_attachable_semantic_tokens
    }
}
