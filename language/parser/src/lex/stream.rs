use std::sync::Arc;

use destack_ast::{TokenSpan, TokenType};
use destack_source::{File, LanguageType, Span};

use super::lex::is_semantic;
use super::lexer::{Lexer, LexerSnapshot, TreeState};

/// Snapshot of token stream state for speculative parsing.
#[derive(Debug, Clone)]
pub struct TokenStreamMark {
    /// The lexer snapshot for restoring positions and stacks.
    pub(super) lexer: LexerSnapshot,
    /// The number of semantic tokens captured in the mark.
    pub(super) tokens_len: usize,
    /// The number of side tokens captured in the mark.
    pub(super) side_tokens_len: usize,
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
    /// The cached next non newline token indexes.
    next_non_newline: Vec<u32>,
    /// The cached matching pair indexes for delimiters.
    matching_pairs: Vec<u32>,
    /// The first token index that still needs a next non newline update.
    pending_non_newline_start: usize,
    /// The stack of open parenthesis token indexes.
    paren_stack: Vec<usize>,
    /// The stack of open brace token indexes.
    brace_stack: Vec<usize>,
    /// The stack of open bracket token indexes.
    bracket_stack: Vec<usize>,
    /// Whether EOF has been reached.
    is_finished: bool,
    /// The cached EOF token, when available.
    eof_token: Option<TokenSpan>,
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
            next_non_newline: Vec::new(),
            matching_pairs: Vec::new(),
            pending_non_newline_start: 0,
            paren_stack: Vec::new(),
            brace_stack: Vec::new(),
            bracket_stack: Vec::new(),
            is_finished: false,
            eof_token: None,
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

    /// Return true once EOF has been reached.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.is_finished
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
        }
    }

    /// Restore token stream state from a snapshot.
    pub fn restore(&mut self, mark: TokenStreamMark) {
        // restore lexer state and truncate token buffers
        self.lexer.restore(mark.lexer);
        self.tokens.truncate(mark.tokens_len);
        self.side_tokens.truncate(mark.side_tokens_len);
        self.lexer.tokens.truncate(mark.tokens_len);
        self.lexer.side_tokens.truncate(mark.side_tokens_len);

        // rebuild caches and stacks to match the restored tokens
        self.rebuild_indexes();

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

        // reset caches and stacks for any follow-up access
        self.next_non_newline.clear();
        self.matching_pairs.clear();
        self.pending_non_newline_start = 0;
        self.paren_stack.clear();
        self.brace_stack.clear();
        self.bracket_stack.clear();

        (tokens, side_tokens)
    }

    /// Look up the next non-newline token index from a start index.
    pub fn next_non_newline_index_from(&mut self, start: usize) -> usize {
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

    /// Return the matching close token index for an opening token, if known.
    pub fn matching_pair(&mut self, index: usize) -> Option<usize> {
        self.ensure_token(index);

        let value = self.matching_pairs.get(index).copied().unwrap_or(u32::MAX);
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

        // advance lexer and build the token span
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

        // route semantic vs side tokens
        if is_semantic(token.ty) {
            self.push_semantic_token(token_span);
        } else {
            self.side_tokens.push(token_span);
            self.lexer.side_tokens.push(token_span);
        }

        // track EOF state
        if token.ty == TokenType::End {
            self.is_finished = true;
            self.eof_token = Some(token_span);
        }
    }

    /// Push a semantic token and update indexes.
    fn push_semantic_token(&mut self, token_span: TokenSpan) {
        // add token and cache slots
        let token_index = self.tokens.len();
        self.tokens.push(token_span);
        self.lexer.tokens.push(token_span);
        self.next_non_newline.push(u32::MAX);
        self.matching_pairs.push(u32::MAX);

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

    /// Rebuild cached indexes and stacks from current tokens.
    fn rebuild_indexes(&mut self) {
        // reset caches and stacks
        self.next_non_newline.clear();
        self.matching_pairs.clear();
        self.paren_stack.clear();
        self.brace_stack.clear();
        self.bracket_stack.clear();
        self.pending_non_newline_start = 0;

        // recompute next non newline and matching pairs
        for (index, token) in self.tokens.iter().enumerate() {
            self.next_non_newline.push(u32::MAX);
            self.matching_pairs.push(u32::MAX);

            if token.token.ty != TokenType::Newline {
                for idx in self.pending_non_newline_start..index {
                    self.next_non_newline[idx] = index as u32;
                }
                self.pending_non_newline_start = index;
            }

            match token.token.ty {
                TokenType::OpenParenthesis => self.paren_stack.push(index),
                TokenType::CloseParenthesis => {
                    if let Some(open) = self.paren_stack.pop() {
                        self.matching_pairs[open] = index as u32;
                    }
                }
                TokenType::OpenBrace => self.brace_stack.push(index),
                TokenType::CloseBrace => {
                    if let Some(open) = self.brace_stack.pop() {
                        self.matching_pairs[open] = index as u32;
                    }
                }
                TokenType::OpenBracket => self.bracket_stack.push(index),
                TokenType::CloseBracket => {
                    if let Some(open) = self.bracket_stack.pop() {
                        self.matching_pairs[open] = index as u32;
                    }
                }
                _ => {}
            }
        }

        if self.is_finished
            && let Some(last_index) = self.tokens.len().checked_sub(1)
        {
            self.next_non_newline[last_index] = self.tokens.len() as u32;
        }
    }
}
