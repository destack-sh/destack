use std::sync::Arc;

use destack_dir::{Comment, Token, TokenRange, TokenSpan, TokenType};
use destack_source::{File, LanguageType, Span};

use super::lexer::{Lexer, ParserTriviaMode};
use super::trivia::{CommentRetention, TriviaComment, retained_comment};

/// Result of lexing one complete source file.
#[derive(Debug)]
pub struct LexResult {
    /// The semantic tokens (identifiers, keywords, literals, operators).
    pub tokens: Vec<TokenSpan>,
    /// The non-semantic tokens (whitespace, comments).
    pub side_tokens: Vec<TokenSpan>,
    /// The structured comments collected during lexing.
    pub comments: Vec<Comment>,
    /// The end-of-file token.
    pub eof_token: TokenSpan,
}

/// Return whether a token is semantic, not whitespace or comment.
#[inline]
pub fn is_semantic(token_type: TokenType) -> bool {
    !matches!(
        token_type,
        TokenType::Newline
            | TokenType::Whitespace
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

impl Lexer {
    /// Return one regular semantic token from the live lexer cursor.
    pub(crate) fn next_semantic_token(&mut self) -> TokenSpan {
        loop {
            if self.is_finished {
                return self.eof_token.unwrap_or_else(|| TokenSpan {
                    token: Token::end(),
                    span: Span::new(
                        self.file_id(),
                        self.position() as u32,
                        self.position() as u32,
                    ),
                });
            }

            let start = self.position() as u32;
            let token = self.advance();
            let token_span = TokenSpan {
                token,
                span: Span {
                    file: self.file_id(),
                    start,
                    end: start + token.len(),
                },
            };

            if is_semantic(token.ty()) {
                let token_span = self.prepare_semantic_token(token_span);
                if token.ty() == TokenType::End {
                    self.is_finished = true;
                    self.eof_token = Some(token_span);
                }

                return token_span;
            }

            let has_line_terminator = self.side_token_had_line_terminator();
            self.push_side_token(token_span, has_line_terminator);
        }
    }

    /// Lex the input string into semantic tokens, side tokens, and the end-of-sequence Token.
    /// Semantic tokens are identifiers, keywords, literals, operators.
    /// Side tokens are whitespace and comments.
    pub fn lex(
        file: Arc<File>,
        language: LanguageType,
    ) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
        let result = Self::lex_file(file, language);
        (result.tokens, result.side_tokens, result.eof_token)
    }

    /// Lex the input string and return token buffers.
    pub fn lex_file(file: Arc<File>, language: LanguageType) -> LexResult {
        let mut lexer = Lexer::new(file, language);
        lexer.set_trivia_mode(ParserTriviaMode::Full);
        lexer.lex_to_result()
    }

    /// Lex the input string with trivia retention configured.
    pub fn lex_with_options(
        file: Arc<File>,
        language: LanguageType,
        trivia_mode: ParserTriviaMode,
    ) -> LexResult {
        let mut lexer = Lexer::new(file, language);
        lexer.set_trivia_mode(trivia_mode);
        lexer.lex_to_result()
    }

    /// Finish lexing and extract the stream buffers.
    fn lex_to_result(&mut self) -> LexResult {
        let eof_token = self.eof_token();
        let comments = self.take_trivia_comments();
        let (tokens, side_tokens) = self.take_tokens();

        LexResult {
            tokens,
            side_tokens,
            comments,
            eof_token,
        }
    }

    /// Set trivia retention before lexing begins.
    #[inline]
    pub fn set_trivia_mode(&mut self, trivia_mode: ParserTriviaMode) {
        debug_assert!(
            self.tokens.is_empty() && self.side_tokens.is_empty(),
            "trivia retention must be configured before lexing starts"
        );
        self.trivia_mode = trivia_mode;
    }

    /// Set trivia retention during speculative cursor movement.
    #[inline]
    pub(crate) fn set_cursor_trivia_mode(&mut self, trivia_mode: ParserTriviaMode) {
        self.trivia_mode = trivia_mode;
    }

    /// Take trivia comments collected during lexing.
    #[inline]
    pub(crate) fn take_trivia_comments(&mut self) -> Vec<TriviaComment> {
        self.trivia.take_comments()
    }

    /// Drain trivia comments collected during lexing into one output buffer.
    #[inline]
    pub(crate) fn drain_trivia_comments_into(&mut self, comments: &mut Vec<TriviaComment>) {
        self.trivia.drain_comments_into(comments);
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

        debug_assert!(false, "lexer must produce an eof token after lexing to end");

        let position = self.position() as u32;
        TokenSpan {
            token: Token::end(),
            span: Span::new(self.file_id(), position, position),
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
        let mut tokens = std::mem::take(&mut self.tokens);
        let mut side_tokens = std::mem::take(&mut self.side_tokens);
        tokens.shrink_to_fit();
        side_tokens.shrink_to_fit();

        // reset token stream flags for any follow-up access
        self.pending_line_terminator_before_next = false;

        (tokens, side_tokens)
    }

    /// Drain side tokens produced so far into one compact output buffer.
    #[inline]
    pub(crate) fn drain_side_token_ranges_into(&mut self, side_tokens: &mut Vec<TokenRange>) {
        side_tokens.extend(self.side_tokens.drain(..).map(TokenRange::from_token_span));
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
    pub(super) fn lex_one(&mut self) {
        let start = self.position() as u32;
        let token = self.advance();
        let token_span = TokenSpan {
            token,
            span: Span {
                file: self.file_id(),
                start,
                end: start + token.len(),
            },
        };

        if is_semantic(token.ty()) {
            self.push_semantic_token(token_span);
        } else {
            let has_line_terminator = self.side_token_had_line_terminator();
            self.push_side_token(token_span, has_line_terminator);
        }

        if token.ty() == TokenType::End {
            self.is_finished = true;
            self.eof_token = Some(token_span);
        }
    }

    /// Push a side token and update stream flags that depend on side tokens.
    #[inline]
    pub(super) fn push_side_token(&mut self, token_span: TokenSpan, has_line_terminator: bool) {
        if self.trivia_mode.keeps_comments() {
            match token_span.token.ty() {
                TokenType::LineComment | TokenType::DocLineComment => {
                    self.push_line_comment(token_span, has_line_terminator);
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    self.push_block_comment(token_span, has_line_terminator);
                }
                TokenType::Newline => {
                    self.trivia.handle_newline(token_span.span.start);
                }
                _ => {}
            }
        }

        if self.trivia_mode.keeps_side_tokens() {
            self.side_tokens.push(token_span);
        }
        if has_line_terminator || token_span.token.ty() == TokenType::Newline {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push a semantic token and update stream state.
    pub(super) fn push_semantic_token(&mut self, mut token_span: TokenSpan) {
        token_span = self.prepare_semantic_token(token_span);
        self.tokens.push(token_span);
    }

    /// Prepare one semantic token for parser consumption.
    pub(super) fn prepare_semantic_token(&mut self, mut token_span: TokenSpan) -> TokenSpan {
        token_span.token = token_span
            .token
            .with_on_new_line(self.pending_line_terminator_before_next);
        self.pending_line_terminator_before_next = false;

        if self.trivia_mode.keeps_comments() {
            self.trivia.handle_token(token_span);
        }

        token_span
    }

    /// Push one retained line comment.
    fn push_line_comment(&mut self, token_span: TokenSpan, has_line_terminator: bool) {
        let raw_comment = self.get_span_str(token_span.span);
        let retention = retained_comment(self.trivia_mode, token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { content, .. } => {
                self.trivia.add_line_comment(token_span, content);
            }
            CommentRetention::Skip => {
                self.push_skipped_comment_boundary(token_span, has_line_terminator);
            }
        }
    }

    /// Push one retained block comment.
    fn push_block_comment(&mut self, token_span: TokenSpan, has_line_terminator: bool) {
        let raw_comment = self.get_span_str(token_span.span);
        let retention = retained_comment(self.trivia_mode, token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { kind, content } => {
                self.trivia.add_block_comment(token_span, kind, content);
            }
            CommentRetention::Skip => {
                self.push_skipped_comment_boundary(token_span, has_line_terminator);
            }
        }
    }

    /// Push one skipped comment boundary.
    fn push_skipped_comment_boundary(&mut self, token_span: TokenSpan, has_line_terminator: bool) {
        if has_line_terminator {
            self.trivia.handle_newline(token_span.span.start);
        } else {
            self.trivia.handle_skipped_side_token(token_span.span.start);
        }
    }
}
