use std::sync::Arc;

use tspp_dir::{Comment, Token, TokenSpan, TokenType};
use tspp_source::File;

use super::comment::CommentDecision;
use super::lexer::{CommentRetention, Lexer};

/// Result of lexing one complete source file.
#[derive(Debug)]
pub struct LexResult {
    /// The semantic tokens (identifiers, keywords, literals, operators).
    pub tokens: Vec<TokenSpan>,
    /// The structured comments collected during lexing.
    pub comments: Vec<Comment>,
    /// The end-of-file token.
    pub eof_token: TokenSpan,
}

impl Lexer {
    /// Lex one file into semantic tokens and its end token.
    pub fn lex(file: Arc<File>) -> (Vec<TokenSpan>, TokenSpan) {
        let result = Self::lex_with_comment_retention(file, CommentRetention::Ignore);

        (result.tokens, result.eof_token)
    }

    /// Lex one file with all source comments.
    pub fn lex_file(file: Arc<File>) -> LexResult {
        Self::lex_with_comment_retention(file, CommentRetention::All)
    }

    /// Lex one file with explicit comment retention.
    pub fn lex_with_comment_retention(
        file: Arc<File>,
        comment_retention: CommentRetention,
    ) -> LexResult {
        let mut lexer = Lexer::new(file);
        lexer.set_comment_retention(comment_retention);

        lexer.lex_to_result()
    }

    /// Finish lexing and extract the stream buffers.
    fn lex_to_result(&mut self) -> LexResult {
        let eof_token = self.eof_token_span();
        let comments = self.take_comments();
        let tokens = self.take_token_spans();

        LexResult {
            tokens,
            comments,
            eof_token,
        }
    }

    /// Set comment retention before lexing begins.
    #[inline]
    pub fn set_comment_retention(&mut self, comment_retention: CommentRetention) {
        debug_assert!(
            self.tokens.is_empty(),
            "comment retention must be configured before lexing starts"
        );
        self.comment_retention = comment_retention;
    }

    /// Take comments collected during lexing.
    #[inline]
    pub(crate) fn take_comments(&mut self) -> Vec<Comment> {
        self.comments.take_comments()
    }

    /// Return the EOF token span, lexing until the end if needed.
    pub fn eof_token_span(&mut self) -> TokenSpan {
        TokenSpan::new(self.eof_token(), self.tokenizer.file_id())
    }

    /// Return the EOF token, lexing until the end if needed.
    pub fn eof_token(&mut self) -> Token {
        self.lex_to_end()
    }

    /// Lex every remaining token and return EOF.
    pub fn lex_to_end(&mut self) -> Token {
        // return the previously reached end of source
        if let Some(eof_token) = self.eof_token {
            return eof_token;
        }

        loop {
            if let Some(token) = self.lex_token()
                && token.is(TokenType::End)
            {
                return token;
            }
        }
    }

    /// Lex through the first semantic token at or after one source position.
    pub(crate) fn lex_through(mut self, position: u32) -> (Vec<Token>, Vec<Comment>) {
        // lex through the synchronization token
        loop {
            let Some(token) = self.lex_token() else {
                continue;
            };
            if token.start() >= position || token.is(TokenType::End) {
                break;
            }
        }

        let comments = self.comments.take_comments();

        (self.tokens, comments)
    }

    /// Return owned semantic token spans and leave the stream empty.
    pub fn take_token_spans(&mut self) -> Vec<TokenSpan> {
        let file_id = self.tokenizer.file_id();
        self.take_tokens()
            .into_iter()
            .map(|token| TokenSpan::new(token, file_id))
            .collect()
    }

    /// Return owned semantic tokens and leave the stream empty.
    pub fn take_tokens(&mut self) -> Vec<Token> {
        // ensure all tokens are available
        self.lex_to_end();

        // drain token buffers
        let tokens = std::mem::take(&mut self.tokens);

        // reset token stream flags for any follow-up access
        self.pending_line_terminator_before_next = false;

        tokens
    }

    /// Lex one raw token and return it when parser-visible.
    fn lex_token(&mut self) -> Option<Token> {
        let token = self.tokenizer.read_source_token();

        // retain one parser-visible token
        if token.is_semantic() {
            self.push_semantic_token(token);
            if token.is(TokenType::End) {
                self.eof_token = Some(token);
            }

            return Some(token);
        }

        // process raw trivia and retained comments
        let has_line_terminator = self.trivia_token_has_line_terminator();
        self.record_trivia(token, has_line_terminator);

        None
    }

    /// Retain one semantic token with its leading line state.
    fn push_semantic_token(&mut self, token: Token) {
        let token = token.with_on_new_line(self.pending_line_terminator_before_next);
        self.pending_line_terminator_before_next = false;

        if self.comment_retention.retains_comments() {
            self.comments.record_token(token.ty(), token.start());
        }

        self.tokens.push(token);
    }

    /// Record parser-visible state from one trivia token.
    #[inline]
    fn record_trivia(&mut self, token: Token, has_line_terminator: bool) {
        let token_type = token.ty();

        if self.comment_retention.retains_comments() {
            match token_type {
                TokenType::LineComment | TokenType::DocLineComment => {
                    self.record_line_comment(token, has_line_terminator);
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    self.record_block_comment(token, has_line_terminator);
                }
                TokenType::Newline => {
                    self.comments.record_newline();
                }
                _ => {}
            }
        }

        if has_line_terminator || token_type == TokenType::Newline {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Record one line comment under the active retention policy.
    fn record_line_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.tokenizer.file_id());
        let raw_comment = self.tokenizer.span_str(token_span.span);
        let decision = self
            .comment_retention
            .classify_comment(token_span.token.ty(), raw_comment);

        match decision {
            CommentDecision::Keep { role, .. } => {
                self.comments.add_line_comment(token_span, role);
            }
            CommentDecision::Skip => {
                self.record_skipped_comment(has_line_terminator);
            }
        }
    }

    /// Record one block comment under the active retention policy.
    fn record_block_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.tokenizer.file_id());
        let raw_comment = self.tokenizer.span_str(token_span.span);
        let decision = self
            .comment_retention
            .classify_comment(token_span.token.ty(), raw_comment);

        match decision {
            CommentDecision::Keep { kind, role } => {
                self.comments.add_block_comment(token_span, kind, role);
            }
            CommentDecision::Skip => {
                self.record_skipped_comment(has_line_terminator);
            }
        }
    }

    /// Record one skipped comment in attachment state.
    fn record_skipped_comment(&mut self, has_line_terminator: bool) {
        if has_line_terminator {
            self.comments.record_newline();
        }
    }
}
