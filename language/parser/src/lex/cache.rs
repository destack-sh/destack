use std::str::FromStr;

use destack_ast::{Keyword, TokenSpan, TokenType};
use destack_source::Span;

use super::lex::is_semantic;
use super::lexer::{Lexer, SemanticTokenData};
use super::trivia::TriviaComment;

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

impl Lexer {
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

    /// Take trivia comments collected during lexing.
    #[inline]
    pub(crate) fn take_trivia_comments(&mut self) -> Vec<TriviaComment> {
        self.trivia.take_comments()
    }

    /// Return true once EOF has been reached.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Allow or disallow tree literal lexing.
    #[inline]
    pub fn set_allow_tree_literals(&mut self, allow: bool) {
        self.options.allow_tree_literals = allow;
    }

    /// Enable or disable tree attribute-value string lexing for the next token.
    #[inline]
    pub fn set_tree_attribute_value(&mut self, enabled: bool) {
        self.options.in_tree_attribute_value = enabled;
    }

    /// Return true when tree literal lexing is enabled.
    #[inline]
    pub fn allow_tree_literals(&self) -> bool {
        self.options.allow_tree_literals
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

    /// Lex the next token as a tree child token.
    pub fn next_tree_child(&mut self) -> TokenSpan {
        let start = self.pos as u32;
        let token = self.advance_tree_child();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.file_id,
                start,
                end: start + token.len,
            },
        };

        if is_semantic(token.ty) {
            self.push_semantic_token(token_span);
        } else {
            let has_line_terminator = self.side_token_had_line_terminator();
            self.push_side_token(token_span, has_line_terminator);
        }

        if token.ty == TokenType::End {
            self.is_finished = true;
            self.eof_token = Some(token_span);
        }

        token_span
    }

    /// Commit a current-token re-lex and discard any stale future materialization.
    pub fn commit_current_token_re_lex(&mut self, index: usize, token_span: TokenSpan) {
        debug_assert!(
            index < self.tokens.len(),
            "current token must be materialized"
        );
        debug_assert!(
            index < self.token_data.len(),
            "current token metadata must be materialized"
        );

        let side_tokens_len_before = self.token_data[index].side_tokens_len_before;
        let old_len = self.tokens.len();
        let new_len = index + 1;

        for removed_index in (new_len..old_len).rev() {
            let removed_token = self.tokens[removed_index];
            let removed_metadata = self.token_data[removed_index];

            if self.retain_trivia_tokens {
                if removed_token.token.ty == TokenType::Newline {
                    self.semantic_newline_token_count -= 1;
                } else if removed_token.token.ty != TokenType::End {
                    self.attachable_semantic_token_count -= 1;
                }
            }

            if matches!(
                removed_token.token.ty,
                TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
            ) {
                let matching_open = removed_metadata.matching_pair as usize;
                if matching_open < new_len {
                    self.token_data[matching_open].matching_pair = u32::MAX;
                    self.restore_unmatched_open(matching_open);
                }
            }
        }

        self.drop_truncated_open_delimiters(new_len);
        self.tokens.truncate(new_len);
        self.tokens[index] = token_span;
        self.side_tokens.truncate(side_tokens_len_before);
        self.token_data.truncate(new_len);

        if let Some(metadata) = self.token_data.get_mut(index) {
            metadata.keyword = None;
            metadata.matching_pair = u32::MAX;
        }

        self.trivia
            .truncate_after(token_span.span.start, token_span.token.ty);
        self.pending_line_terminator_before_next = false;
        self.pending_comment_before_next = false;
        self.last_side_token_had_line_terminator = false;
        self.is_finished = false;
        self.eof_token = None;

        self.reset_cursor_after_re_lex(token_span.span.end as usize);
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
                file: self.file_id,
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

        // reset caches and stacks for any follow-up access
        self.token_data.clear();
        self.paren_stack.clear();
        self.brace_stack.clear();
        self.bracket_stack.clear();
        self.pending_line_terminator_before_next = false;
        self.pending_comment_before_next = false;
        self.semantic_newline_token_count = 0;
        self.attachable_semantic_token_count = 0;

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

        // hot fast path: the full token stream is already materialized
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

        self.get_span_str(token.span).as_bytes().contains(&b'\\')
    }

    /// Return true when lexing reached EOF.
    #[inline]
    pub fn is_lexed_to_end(&self) -> bool {
        self.is_finished
    }

    /// Return the matching close token index for an opening token, if known.
    pub fn matching_pair(&mut self, index: usize) -> Option<usize> {
        // hot fast path: the full token stream is already materialized
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

    /// Lex one token and route it through the shared stream update path.
    #[inline]
    fn lex_one(&mut self) {
        let start = self.pos as u32;
        let token = self.advance();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.file_id,
                start,
                end: start + token.len,
            },
        };

        if is_semantic(token.ty) {
            self.push_semantic_token(token_span);
        } else {
            let has_line_terminator = self.side_token_had_line_terminator();
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
        let raw_comment = self.get_span_str(token_span.span).to_string();

        match token_span.token.ty {
            TokenType::LineComment | TokenType::DocLineComment => {
                self.trivia.add_line_comment(token_span, &raw_comment);
            }
            TokenType::BlockComment | TokenType::DocBlockComment => {
                self.trivia.add_block_comment(token_span, &raw_comment);
            }
            _ => {}
        }

        if self.retain_trivia_tokens
            && matches!(
                token_span.token.ty,
                TokenType::LineComment
                    | TokenType::DocLineComment
                    | TokenType::BlockComment
                    | TokenType::DocBlockComment
            )
        {
            self.pending_comment_before_next = true;
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
        let side_tokens_len_before = self.side_tokens.len();
        let has_line_terminator_before = self.pending_line_terminator_before_next;
        let keyword = if token_span.token.ty == TokenType::Identifier {
            keyword_from_identifier(self.get_span_str(token_span.span))
        } else {
            None
        };
        // add token and dense metadata
        self.tokens.push(token_span);
        self.token_data.push(SemanticTokenData::new(
            side_tokens_len_before,
            keyword,
            has_line_terminator_before,
        ));
        if self.retain_trivia_tokens {
            self.token_data[token_index].has_comment_before = self.pending_comment_before_next;
        }
        self.pending_line_terminator_before_next = token_span.token.ty == TokenType::Newline;
        self.pending_comment_before_next = false;
        if self.retain_trivia_tokens {
            if token_span.token.ty == TokenType::Newline {
                self.semantic_newline_token_count += 1;
            } else if token_span.token.ty != TokenType::End {
                self.attachable_semantic_token_count += 1;
            }
        }

        // newline stays trivia-only for comment attachment
        if token_span.token.ty == TokenType::Newline {
            self.trivia.handle_newline(token_span.span.start);
        } else {
            self.trivia.handle_token(token_span);
        }

        // update matching pairs for brackets
        match token_span.token.ty {
            TokenType::OpenParenthesis => self.paren_stack.push(token_index),
            TokenType::CloseParenthesis => {
                if let Some(open) = self.paren_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                    self.token_data[token_index].matching_pair = open as u32;
                }
            }
            TokenType::OpenBrace => self.brace_stack.push(token_index),
            TokenType::CloseBrace => {
                if let Some(open) = self.brace_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                    self.token_data[token_index].matching_pair = open as u32;
                }
            }
            TokenType::OpenBracket => self.bracket_stack.push(token_index),
            TokenType::CloseBracket => {
                if let Some(open) = self.bracket_stack.pop() {
                    self.token_data[open].matching_pair = token_index as u32;
                    self.token_data[token_index].matching_pair = open as u32;
                }
            }
            _ => {}
        }
    }

    /// Return true when any trivia comments were collected.
    #[inline]
    pub fn has_comment_tokens(&self) -> bool {
        self.trivia.has_comments()
    }

    /// Return true when non-newline semantic tokens were seen.
    #[inline]
    pub fn has_attachable_semantic_tokens(&self) -> bool {
        self.attachable_semantic_token_count > 0
    }

    /// Restore one unmatched opening delimiter after suffix truncation.
    fn restore_unmatched_open(&mut self, open_index: usize) {
        match self.tokens[open_index].token.ty {
            TokenType::OpenParenthesis => self.paren_stack.push(open_index),
            TokenType::OpenBrace => self.brace_stack.push(open_index),
            TokenType::OpenBracket => self.bracket_stack.push(open_index),
            _ => {}
        }
    }

    /// Drop unmatched opening delimiters that lived only in the truncated suffix.
    fn drop_truncated_open_delimiters(&mut self, len: usize) {
        while self.paren_stack.peek().is_some_and(|open| open >= len) {
            self.paren_stack.pop();
        }

        while self.brace_stack.peek().is_some_and(|open| open >= len) {
            self.brace_stack.pop();
        }

        while self.bracket_stack.peek().is_some_and(|open| open >= len) {
            self.bracket_stack.pop();
        }
    }
}
