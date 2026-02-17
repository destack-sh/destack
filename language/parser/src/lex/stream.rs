use std::str::FromStr;
use std::sync::Arc;

use destack_ast::{Keyword, TokenSpan, TokenType};
use destack_source::{File, LanguageType, Span};

use super::lex::is_semantic;
use super::lexer::{Lexer, LexerSnapshot, TreeState};

/// Return a keyword for an identifier when it can match keyword shape.
#[inline]
fn keyword_from_identifier(identifier: &str) -> Option<Keyword> {
    // quick reject using identifier length bounds for known keywords
    let length = identifier.len();
    if !(2..=11).contains(&length) {
        return None;
    }

    // quick reject using keyword (length, first byte) pairs before full string matching
    let first = identifier.as_bytes().first().copied()?;
    let can_match_keyword = matches!(
        (length, first),
        (2, b'a' | b'd' | b'i' | b'o' | b't')
            | (3, b'a' | b'f' | b'g' | b'l' | b'n' | b's' | b't' | b'v')
            | (
                4,
                b'c' | b'e' | b'f' | b'g' | b'l' | b'm' | b's' | b't' | b'v' | b'w'
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
    /// The first mutable next non newline index at mark time.
    pub(super) pending_non_newline_start: usize,
    /// The next non newline tail values from pending_non_newline_start onward.
    pub(super) next_non_newline_tail: Vec<u32>,
    /// The open parenthesis stack at mark time.
    pub(super) paren_stack: Vec<usize>,
    /// The open brace stack at mark time.
    pub(super) brace_stack: Vec<usize>,
    /// The open bracket stack at mark time.
    pub(super) bracket_stack: Vec<usize>,
    /// Side trivia start indexes per semantic token from the mark tail.
    pub(super) leading_side_start_tail: Vec<u32>,
    /// Side trivia end indexes per semantic token from the mark tail.
    pub(super) leading_side_end_tail: Vec<u32>,
    /// Prefix counts of semantic tokens that have non whitespace side trivia.
    pub(super) non_whitespace_side_prefix_tail: Vec<u32>,
    /// The pending side trivia start index for the next semantic token.
    pub(super) pending_leading_side_start: usize,
    /// Whether side trivia since the last semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// Whether this stream snapshot has comment style side annotations.
    pub(super) has_comment_side_tokens: bool,
    /// Whether this stream snapshot has semantic newline tokens.
    pub(super) has_semantic_newline_tokens: bool,
    /// Pending synthetic token from operator decomposition.
    pub(super) split_token: Option<TokenSpan>,
    /// Whether the pending split token has already been consumed.
    pub(super) split_token_consumed: bool,
}

/// Cursor information for the first non-newline token from a start index.
#[derive(Debug, Copy, Clone)]
pub struct TokenStreamCursor {
    /// The semantic token index.
    pub index: usize,
    /// The token type at `index`.
    pub token_type: TokenType,
    /// The number of leading newline tokens skipped from `start`.
    pub skipped_newline_count: usize,
    /// Whether a line break appears before the token at `index`.
    pub has_line_break_before: bool,
}

/// Lazy token stream that drives the lexer on demand.
#[derive(Debug)]
pub struct TokenStream {
    /// The underlying lexer.
    lexer: Lexer,
    /// The semantic tokens produced so far.
    tokens: Vec<TokenSpan>,
    /// The side tokens produced so far.
    side_tokens: Vec<TokenSpan>,
    /// Indexes of comment-like side tokens in `side_tokens`.
    comment_side_token_indexes: Vec<u32>,
    /// The semantic token index that each side token belongs to.
    side_owner_token_index: Vec<u32>,
    /// The cached next non newline token indexes.
    next_non_newline: Vec<u32>,
    /// The cached matching pair indexes for delimiters.
    matching_pairs: Vec<u32>,
    /// Cached keyword values for identifier tokens.
    token_keywords: Vec<Option<Keyword>>,
    /// Cached line terminator presence before semantic token indexes.
    line_terminators_before: Vec<bool>,
    /// Side trivia start index before each semantic token.
    leading_side_start_by_token: Vec<u32>,
    /// Side trivia end index before each semantic token.
    leading_side_end_by_token: Vec<u32>,
    /// Prefix counts for semantic tokens that have non whitespace side trivia.
    non_whitespace_side_prefix: Vec<u32>,
    /// Side trivia start index for the next semantic token.
    pending_leading_side_start: usize,
    /// The first token index that still needs a next non newline update.
    pending_non_newline_start: usize,
    /// The stack of open parenthesis token indexes.
    paren_stack: Vec<usize>,
    /// The stack of open brace token indexes.
    brace_stack: Vec<usize>,
    /// The stack of open bracket token indexes.
    bracket_stack: Vec<usize>,
    /// Whether side trivia since the previous semantic token had a line terminator.
    pending_line_terminator_before_next: bool,
    /// Whether any comment style side trivia token was seen.
    has_comment_side_tokens: bool,
    /// Whether any semantic newline token was seen.
    has_semantic_newline_tokens: bool,
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

        // allocate buffers using the same heuristic as the lexer
        let tokens = Vec::with_capacity(estimated_tokens * 3 / 5);
        let side_tokens = Vec::with_capacity(estimated_tokens * 2 / 5);

        // build the stream
        Self {
            lexer: Lexer::new(file, language),
            tokens,
            side_tokens,
            comment_side_token_indexes: Vec::with_capacity(estimated_tokens / 24),
            side_owner_token_index: Vec::with_capacity(estimated_tokens * 2 / 5),
            next_non_newline: Vec::new(),
            matching_pairs: Vec::new(),
            token_keywords: Vec::new(),
            line_terminators_before: Vec::new(),
            leading_side_start_by_token: Vec::new(),
            leading_side_end_by_token: Vec::new(),
            non_whitespace_side_prefix: vec![0],
            pending_leading_side_start: 0,
            pending_non_newline_start: 0,
            paren_stack: Vec::new(),
            brace_stack: Vec::new(),
            bracket_stack: Vec::new(),
            pending_line_terminator_before_next: false,
            has_comment_side_tokens: false,
            has_semantic_newline_tokens: false,
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

    /// Return the owner semantic token index for a side token.
    #[inline]
    pub fn side_owner_token_index(&self, side_index: usize) -> Option<usize> {
        self.side_owner_token_index
            .get(side_index)
            .copied()
            .map(|index| index as usize)
    }

    /// Return all side token owner semantic indexes.
    #[inline]
    pub fn side_owner_token_indexes(&self) -> &[u32] {
        &self.side_owner_token_index
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
        if index >= self.leading_side_start_by_token.len() {
            let side_len = self.side_tokens.len();
            return (side_len, side_len);
        }

        let start = self.leading_side_start_by_token[index] as usize;
        let end = self.leading_side_end_by_token[index] as usize;
        (start, end)
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

    /// Return true when the lexer is inside a tree attribute expression container.
    #[inline]
    pub fn in_tree_attribute_expression(&self) -> bool {
        self.lexer.in_tree_attribute_expression()
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
        let pending_non_newline_start = self.pending_non_newline_start.min(self.tokens.len());
        TokenStreamMark {
            lexer: self.lexer.snapshot(),
            tokens_len: self.tokens.len(),
            side_tokens_len: self.side_tokens.len(),
            comment_side_tokens_len: self.comment_side_token_indexes.len(),
            pending_non_newline_start,
            next_non_newline_tail: self.next_non_newline[pending_non_newline_start..].to_vec(),
            paren_stack: self.paren_stack.clone(),
            brace_stack: self.brace_stack.clone(),
            bracket_stack: self.bracket_stack.clone(),
            leading_side_start_tail: self.leading_side_start_by_token[pending_non_newline_start..]
                .to_vec(),
            leading_side_end_tail: self.leading_side_end_by_token[pending_non_newline_start..]
                .to_vec(),
            non_whitespace_side_prefix_tail: self.non_whitespace_side_prefix
                [pending_non_newline_start + 1..]
                .to_vec(),
            pending_leading_side_start: self.pending_leading_side_start,
            pending_line_terminator_before_next: self.pending_line_terminator_before_next,
            has_comment_side_tokens: self.has_comment_side_tokens,
            has_semantic_newline_tokens: self.has_semantic_newline_tokens,
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
            pending_non_newline_start,
            next_non_newline_tail,
            paren_stack,
            brace_stack,
            bracket_stack,
            leading_side_start_tail,
            leading_side_end_tail,
            non_whitespace_side_prefix_tail,
            pending_leading_side_start,
            pending_line_terminator_before_next,
            has_comment_side_tokens,
            has_semantic_newline_tokens,
            split_token,
            split_token_consumed,
        } = mark;

        // restore lexer state and truncate token buffers
        let semantic_tokens_changed = self.tokens.len() != tokens_len;
        let side_tokens_changed = self.side_tokens.len() != side_tokens_len;
        self.lexer.restore(lexer);
        if side_tokens_changed {
            self.side_tokens.truncate(side_tokens_len);
            self.lexer.side_tokens.truncate(side_tokens_len);
            self.side_owner_token_index.truncate(side_tokens_len);
        }
        self.comment_side_token_indexes
            .truncate(comment_side_tokens_len);
        self.pending_line_terminator_before_next = pending_line_terminator_before_next;
        self.pending_leading_side_start = pending_leading_side_start;
        self.has_comment_side_tokens = has_comment_side_tokens;
        self.has_semantic_newline_tokens = has_semantic_newline_tokens;
        self.split_token = split_token;
        self.split_token_consumed = split_token_consumed;

        // fast path: no semantic token changes, caches are still valid
        if !semantic_tokens_changed {
            return;
        }

        self.tokens.truncate(tokens_len);
        self.lexer.tokens.truncate(tokens_len);
        self.token_keywords.truncate(tokens_len);
        self.line_terminators_before.truncate(tokens_len);
        self.leading_side_start_by_token.truncate(tokens_len);
        self.leading_side_end_by_token.truncate(tokens_len);
        self.non_whitespace_side_prefix.truncate(tokens_len + 1);

        // restore next non newline cache and mutable tail cursor
        self.next_non_newline.truncate(tokens_len);
        for (offset, next_non_newline) in next_non_newline_tail.into_iter().enumerate() {
            self.next_non_newline[pending_non_newline_start + offset] = next_non_newline;
        }
        for (offset, side_start) in leading_side_start_tail.into_iter().enumerate() {
            self.leading_side_start_by_token[pending_non_newline_start + offset] = side_start;
        }
        for (offset, side_end) in leading_side_end_tail.into_iter().enumerate() {
            self.leading_side_end_by_token[pending_non_newline_start + offset] = side_end;
        }
        for (offset, prefix) in non_whitespace_side_prefix_tail.into_iter().enumerate() {
            self.non_whitespace_side_prefix[pending_non_newline_start + 1 + offset] = prefix;
        }
        self.pending_non_newline_start = pending_non_newline_start;

        // restore matching pair cache and open delimiter stacks
        self.matching_pairs.truncate(tokens_len);
        for open_index in paren_stack
            .iter()
            .copied()
            .chain(brace_stack.iter().copied())
            .chain(bracket_stack.iter().copied())
        {
            self.matching_pairs[open_index] = u32::MAX;
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
            self.lex_to_end_fast_non_tree();
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
        self.lexer.tokens.clear();
        self.lexer.side_tokens.clear();
        self.side_owner_token_index.clear();
        self.comment_side_token_indexes.clear();

        // reset caches and stacks for any follow-up access
        self.next_non_newline.clear();
        self.matching_pairs.clear();
        self.token_keywords.clear();
        self.line_terminators_before.clear();
        self.leading_side_start_by_token.clear();
        self.leading_side_end_by_token.clear();
        self.non_whitespace_side_prefix.clear();
        self.non_whitespace_side_prefix.push(0);
        self.pending_leading_side_start = 0;
        self.pending_non_newline_start = 0;
        self.paren_stack.clear();
        self.brace_stack.clear();
        self.bracket_stack.clear();
        self.pending_line_terminator_before_next = false;
        self.has_comment_side_tokens = false;
        self.has_semantic_newline_tokens = false;
        self.clear_split_token();

        (tokens, side_tokens)
    }

    /// Return whether trivia before a semantic token index had a line terminator.
    #[inline]
    pub fn line_terminator_before(&mut self, index: usize) -> bool {
        // hot fast path: full token stream is already materialized
        if self.is_finished {
            return self
                .line_terminators_before
                .get(index)
                .copied()
                .unwrap_or(false);
        }

        self.ensure_token(index);
        self.line_terminators_before
            .get(index)
            .copied()
            .unwrap_or(false)
    }

    /// Return the keyword for a semantic token index.
    #[inline]
    pub fn keyword_at(&mut self, index: usize) -> Option<Keyword> {
        self.ensure_token(index);
        self.token_keywords.get(index).copied().unwrap_or(None)
    }

    /// Return true when a semantic token window has any non-whitespace side trivia in cached mode.
    #[inline]
    pub fn has_non_whitespace_side_in_window_cached(
        &self,
        token_window_start: usize,
        token_window_end_exclusive: usize,
    ) -> bool {
        let token_len = self.tokens.len();
        let token_window_start = token_window_start.min(token_len);
        let token_window_end_exclusive = token_window_end_exclusive.min(token_len);
        if token_window_start >= token_window_end_exclusive {
            return false;
        }

        let start = self
            .non_whitespace_side_prefix
            .get(token_window_start)
            .copied()
            .unwrap_or(0);
        let end = self
            .non_whitespace_side_prefix
            .get(token_window_end_exclusive)
            .copied()
            .unwrap_or(start);
        end > start
    }

    /// Return true when a semantic token window has any non-whitespace side trivia.
    #[inline]
    pub fn has_non_whitespace_side_in_window(
        &mut self,
        token_window_start: usize,
        token_window_end_exclusive: usize,
    ) -> bool {
        if self.is_finished {
            return self.has_non_whitespace_side_in_window_cached(
                token_window_start,
                token_window_end_exclusive,
            );
        }

        if token_window_end_exclusive == 0 {
            return false;
        }

        self.ensure_token(token_window_end_exclusive.saturating_sub(1));
        self.has_non_whitespace_side_in_window_cached(
            token_window_start,
            token_window_end_exclusive,
        )
    }

    /// Look up the next non-newline token index from a start index.
    pub fn next_non_newline_index_from(&mut self, start: usize) -> usize {
        // hot fast path: full token stream is already materialized
        if self.is_finished {
            if start >= self.tokens.len() {
                return self.tokens.len();
            }

            if self.tokens[start].token.ty != TokenType::Newline {
                return start;
            }

            let next = self
                .next_non_newline
                .get(start)
                .copied()
                .unwrap_or(u32::MAX);
            if next == u32::MAX {
                return self.tokens.len();
            }

            return next as usize;
        }

        // ensure the starting token exists
        self.ensure_token(start);

        // return immediately if the start is already non newline
        if let Some(token) = self.tokens.get(start) {
            if token.token.ty != TokenType::Newline {
                return start;
            }
        } else {
            return self.tokens.len();
        }

        // chase the cached next non newline pointer
        loop {
            let Some(next) = self.next_non_newline.get(start).copied() else {
                return self.tokens.len();
            };

            if next != u32::MAX {
                return next as usize;
            }

            if self.is_finished {
                return self.tokens.len();
            }

            self.lex_next();
        }
    }

    /// Return cursor information for the first non-newline token from a start index.
    #[inline]
    pub fn non_newline_cursor_from(&mut self, start: usize) -> TokenStreamCursor {
        // hot fast path: full token stream is already materialized
        if self.is_finished {
            return self.non_newline_cursor_from_cached(start);
        }

        self.ensure_token(start);
        self.non_newline_cursor_from_materialized(start)
    }

    /// Return cursor information for the first non-newline token from a start index.
    #[inline]
    fn non_newline_cursor_from_cached(&self, start: usize) -> TokenStreamCursor {
        let len = self.tokens.len();
        if start >= len {
            return TokenStreamCursor {
                index: len,
                token_type: TokenType::End,
                skipped_newline_count: 0,
                has_line_break_before: false,
            };
        }

        let token = self.tokens[start];
        if token.token.ty != TokenType::Newline {
            let has_line_break_before = self
                .line_terminators_before
                .get(start)
                .copied()
                .unwrap_or(false);
            return TokenStreamCursor {
                index: start,
                token_type: token.token.ty,
                skipped_newline_count: 0,
                has_line_break_before,
            };
        }

        let next = self
            .next_non_newline
            .get(start)
            .copied()
            .unwrap_or(u32::MAX);
        if next == u32::MAX {
            return TokenStreamCursor {
                index: len,
                token_type: TokenType::End,
                skipped_newline_count: len.saturating_sub(start),
                has_line_break_before: true,
            };
        }

        let next_index = next as usize;
        let next_token_type = self
            .tokens
            .get(next_index)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End);
        TokenStreamCursor {
            index: next_index,
            token_type: next_token_type,
            skipped_newline_count: next_index.saturating_sub(start),
            has_line_break_before: true,
        }
    }

    /// Return cursor information for the first non-newline token from a start index.
    #[inline]
    fn non_newline_cursor_from_materialized(&mut self, start: usize) -> TokenStreamCursor {
        let len = self.tokens.len();
        if start >= len {
            return TokenStreamCursor {
                index: len,
                token_type: TokenType::End,
                skipped_newline_count: 0,
                has_line_break_before: false,
            };
        }

        let token = self.tokens[start];
        if token.token.ty != TokenType::Newline {
            let has_line_break_before = self
                .line_terminators_before
                .get(start)
                .copied()
                .unwrap_or(false);
            return TokenStreamCursor {
                index: start,
                token_type: token.token.ty,
                skipped_newline_count: 0,
                has_line_break_before,
            };
        }

        let next_index = self.next_non_newline_index_from(start);
        if next_index >= self.tokens.len() {
            return TokenStreamCursor {
                index: self.tokens.len(),
                token_type: TokenType::End,
                skipped_newline_count: next_index.saturating_sub(start),
                has_line_break_before: true,
            };
        }

        let next_token_type = self.tokens[next_index].token.ty;
        TokenStreamCursor {
            index: next_index,
            token_type: next_token_type,
            skipped_newline_count: next_index.saturating_sub(start),
            has_line_break_before: true,
        }
    }

    /// Return the matching close token index for an opening token, if known.
    pub fn matching_pair(&mut self, index: usize) -> Option<usize> {
        // hot fast path: full token stream is already materialized
        if self.is_finished {
            let value = self.matching_pairs.get(index).copied().unwrap_or(u32::MAX);
            if value == u32::MAX {
                return None;
            }

            return Some(value as usize);
        }

        self.ensure_token(index);

        let value = self.matching_pairs.get(index).copied().unwrap_or(u32::MAX);
        if value == u32::MAX {
            return None;
        }

        Some(value as usize)
    }

    /// Return the matching close token index for an opening token, lexing ahead if needed.
    pub fn matching_pair_or_lex(&mut self, index: usize) -> Option<usize> {
        self.ensure_token(index);

        let token = self.tokens.get(index)?;

        if !matches!(
            token.token.ty,
            TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
        ) {
            return None;
        }

        loop {
            let value = self.matching_pairs.get(index).copied().unwrap_or(u32::MAX);
            if value != u32::MAX {
                return Some(value as usize);
            }

            if self.is_finished {
                return None;
            }

            self.lex_next();
        }
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
    fn lex_to_end_fast_non_tree(&mut self) {
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
        let side_index = self.side_tokens.len() as u32;
        if matches!(
            token_span.token.ty,
            TokenType::LineComment
                | TokenType::DocLineComment
                | TokenType::BlockComment
                | TokenType::DocBlockComment
        ) {
            self.has_comment_side_tokens = true;
            self.comment_side_token_indexes.push(side_index);
        }

        self.side_tokens.push(token_span);
        self.side_owner_token_index.push(u32::MAX);
        if has_line_terminator {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push a semantic token and update indexes.
    fn push_semantic_token(&mut self, token_span: TokenSpan) {
        let token_index = self.tokens.len();
        let has_line_terminator_before = self.pending_line_terminator_before_next;
        let leading_side_start = self.pending_leading_side_start as u32;
        let leading_side_end = self.side_tokens.len() as u32;
        let mut has_non_whitespace_side = false;
        let pending_side_start = self.pending_leading_side_start;
        let pending_side_end = self.side_tokens.len();
        for side_index in pending_side_start..pending_side_end {
            self.side_owner_token_index[side_index] = token_index as u32;
            if self.side_tokens[side_index].token.ty != TokenType::Whitespace {
                has_non_whitespace_side = true;
            }
        }

        // add token and cache slots
        self.tokens.push(token_span);
        self.next_non_newline.push(u32::MAX);
        self.matching_pairs.push(u32::MAX);
        let keyword = if token_span.token.ty == TokenType::Identifier {
            keyword_from_identifier(self.lexer.get_span_str(token_span.span))
        } else {
            None
        };
        self.token_keywords.push(keyword);
        self.line_terminators_before
            .push(has_line_terminator_before);
        self.leading_side_start_by_token.push(leading_side_start);
        self.leading_side_end_by_token.push(leading_side_end);
        let previous_non_whitespace_side_prefix =
            self.non_whitespace_side_prefix.last().copied().unwrap_or(0);
        self.non_whitespace_side_prefix
            .push(previous_non_whitespace_side_prefix + has_non_whitespace_side as u32);
        self.pending_leading_side_start = self.side_tokens.len();
        self.pending_line_terminator_before_next = token_span.token.ty == TokenType::Newline;
        if token_span.token.ty == TokenType::Newline {
            self.has_semantic_newline_tokens = true;
        }

        // update lexer context for regex and tree rules
        self.lexer.track_semantic_token(token_span);

        // update next non newline for pending tokens
        if token_span.token.ty != TokenType::Newline {
            for idx in self.pending_non_newline_start..token_index {
                self.next_non_newline[idx] = token_index as u32;
            }
            self.pending_non_newline_start = token_index;
        }

        // update matching pairs for brackets
        match token_span.token.ty {
            TokenType::OpenParenthesis => self.paren_stack.push(token_index),
            TokenType::CloseParenthesis => {
                if let Some(open) = self.paren_stack.pop() {
                    self.matching_pairs[open] = token_index as u32;
                }
            }
            TokenType::OpenBrace => self.brace_stack.push(token_index),
            TokenType::CloseBrace => {
                if let Some(open) = self.brace_stack.pop() {
                    self.matching_pairs[open] = token_index as u32;
                }
            }
            TokenType::OpenBracket => self.bracket_stack.push(token_index),
            TokenType::CloseBracket => {
                if let Some(open) = self.bracket_stack.pop() {
                    self.matching_pairs[open] = token_index as u32;
                }
            }
            _ => {}
        }

        // fill EOF next non newline once the end is reached
        if token_span.token.ty == TokenType::End {
            self.next_non_newline[token_index] = (token_index + 1) as u32;
        }
    }

    /// Return true when comment style side annotation tokens were seen.
    #[inline]
    pub fn has_comment_trivia_tokens(&self) -> bool {
        self.has_comment_side_tokens
    }

    /// Return true when semantic newline tokens were seen.
    #[inline]
    pub fn has_blank_trivia_tokens(&self) -> bool {
        self.has_semantic_newline_tokens
    }
}
