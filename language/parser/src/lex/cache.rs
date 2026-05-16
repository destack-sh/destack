use std::str::FromStr;

use destack_dir::{Keyword, TokenSpan, TokenType};
use destack_source::Span;

use super::lex::is_semantic;
use super::lexer::Lexer;
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
                b'a' | b'b'
                    | b'c'
                    | b'f'
                    | b'i'
                    | b'k'
                    | b'l'
                    | b'm'
                    | b'n'
                    | b's'
                    | b't'
                    | b'u'
                    | b'w'
                    | b'y'
            )
            | (6, b'a' | b'd' | b'e' | b'i' | b'p' | b'r' | b's' | b't')
            | (7, b'a' | b'd' | b'e' | b'f' | b'n' | b'p' | b'v')
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

    /// Read the next semantic token from the current lexer cursor.
    pub fn next_token(&mut self) -> TokenSpan {
        let token_index = self.tokens.len();

        while !self.is_finished && self.tokens.len() <= token_index {
            self.lex_next();
        }

        if let Some(token) = self.tokens.get(token_index) {
            return *token;
        }

        if let Some(token) = self.eof_token {
            return token;
        }

        panic!("lexer must produce an eof token after finishing");
    }

    /// Lex the next token as a tree child token.
    pub fn next_tree_child(&mut self) -> TokenSpan {
        let start = self.position() as u32;
        let token = self.advance_tree_child();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.file_id(),
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

    /// Replace the current stream-tail token after tokenization refines its kind.
    pub fn replace_current_token(&mut self, token_span: TokenSpan) {
        let token = self
            .tokens
            .last_mut()
            .expect("current stream-tail token must exist");
        *token = token_span;
    }

    /// Get the EOF token, lexing until the end if needed.
    pub fn eof_token(&mut self) -> TokenSpan {
        // lex until EOF is reached
        while !self.is_finished {
            self.lex_next();
        }

        if let Some(token) = self.eof_token {
            return token;
        }

        panic!("lexer must produce an eof token after lexing to end");
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

        // reset token stream flags for any follow-up access
        self.pending_line_terminator_before_next = false;
        self.attachable_semantic_token_count = 0;

        (tokens, side_tokens)
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
        let start = self.position() as u32;
        let token = self.advance();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.file_id(),
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
            TokenType::Newline => {
                self.trivia.handle_newline(token_span.span.start);
            }
            _ => {}
        }

        if self.retain_trivia_tokens {
            self.side_tokens.push(token_span);
        }
        if has_line_terminator || token_span.token.ty == TokenType::Newline {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push a semantic token and update stream state.
    fn push_semantic_token(&mut self, mut token_span: TokenSpan) {
        // attach line boundary and store token
        token_span.token = token_span
            .token
            .with_on_new_line(self.pending_line_terminator_before_next);
        self.tokens.push(token_span);
        self.pending_line_terminator_before_next = false;
        if token_span.token.ty == TokenType::At {
            self.has_at = true;
        }
        if self.retain_trivia_tokens && token_span.token.ty != TokenType::End {
            self.attachable_semantic_token_count += 1;
        }

        self.trivia.handle_token(token_span);
    }

    /// Return true when any trivia comments were collected.
    #[inline]
    pub fn has_comment_tokens(&self) -> bool {
        self.trivia.has_comments()
    }

    /// Return true when attachable semantic tokens were seen.
    #[inline]
    pub fn has_attachable_semantic_tokens(&self) -> bool {
        self.attachable_semantic_token_count > 0
    }
}
