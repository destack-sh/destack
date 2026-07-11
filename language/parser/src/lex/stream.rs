use std::sync::Arc;

use destack_dir::{Comment, Token, TokenSpan, TokenType};
use destack_source::File;

use super::lexer::{Lexer, ParserTriviaMode};
use super::trivia::CommentRetention;

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

impl Lexer {
    /// Lex one file into semantic tokens, side tokens, and its end token.
    ///
    /// Semantic tokens are identifiers, keywords, literals, operators.
    /// Side tokens are whitespace and comments.
    pub fn lex(file: Arc<File>) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
        let result = Self::lex_file(file);
        (result.tokens, result.side_tokens, result.eof_token)
    }

    /// Lex one file and return its token buffers.
    pub fn lex_file(file: Arc<File>) -> LexResult {
        let mut lexer = Lexer::new(file);
        lexer.set_trivia_mode(ParserTriviaMode::Full);
        lexer.lex_to_result()
    }

    /// Lex one file with explicit trivia retention.
    pub fn lex_with_options(file: Arc<File>, trivia_mode: ParserTriviaMode) -> LexResult {
        let mut lexer = Lexer::new(file);
        lexer.set_trivia_mode(trivia_mode);
        lexer.lex_to_result()
    }

    /// Finish lexing and extract the stream buffers.
    fn lex_to_result(&mut self) -> LexResult {
        let eof_token = self.eof_token_span();
        let comments = self.take_comments();
        let (tokens, side_tokens) = self.take_token_spans();

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

    /// Take trivia comments collected during lexing.
    #[inline]
    pub(crate) fn take_comments(&mut self) -> Vec<Comment> {
        self.trivia.take_comments()
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
        loop {
            if let Some(eof_token) = self.eof_token {
                return eof_token;
            }

            self.lex_next();
        }
    }

    /// Return owned token span buffers and leave the stream empty.
    pub fn take_token_spans(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        let file_id = self.tokenizer.file_id();
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

    /// Return owned compact token buffers and leave the stream empty.
    pub fn take_tokens(&mut self) -> (Vec<Token>, Vec<Token>) {
        // ensure all tokens are available
        self.lex_to_end();

        // drain token buffers
        let tokens = std::mem::take(&mut self.tokens);
        let side_tokens = std::mem::take(&mut self.side_tokens);

        // reset token stream flags for any follow-up access
        self.pending_line_terminator_before_next = false;

        (tokens, side_tokens)
    }

    /// Lex one source token and update the retained token stream.
    fn lex_next(&mut self) {
        let token = self.tokenizer.read_source_token();

        if token.is_semantic() {
            self.push_semantic_token(token);
        } else {
            let has_line_terminator = self.side_token_has_line_terminator();
            self.push_side_token(token, has_line_terminator);
        }

        if token.is(TokenType::End) {
            self.eof_token = Some(token);
        }
    }

    /// Retain one semantic token with its leading line state.
    fn push_semantic_token(&mut self, token: Token) {
        let token = token.with_on_new_line(self.pending_line_terminator_before_next);
        self.pending_line_terminator_before_next = false;

        if self.trivia_mode.keeps_comments() {
            self.trivia.record_token(token.ty(), token.start());
        }

        self.tokens.push(token);
    }

    /// Push a side token and update stream flags that depend on side tokens.
    #[inline]
    fn push_side_token(&mut self, token: Token, has_line_terminator: bool) {
        self.record_side_token_trivia(token, has_line_terminator);

        if self.trivia_mode.keeps_side_tokens() {
            self.side_tokens.push(token);
        }
    }

    /// Update parser-visible trivia state for one side token.
    #[inline]
    fn record_side_token_trivia(&mut self, token: Token, has_line_terminator: bool) {
        let token_type = token.ty();

        if self.trivia_mode.keeps_comments() {
            match token_type {
                TokenType::LineComment | TokenType::DocLineComment => {
                    self.push_line_comment(token, has_line_terminator);
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    self.push_block_comment(token, has_line_terminator);
                }
                TokenType::Newline => {
                    self.trivia.record_newline(token.start());
                }
                _ => {}
            }
        }

        if has_line_terminator || token_type == TokenType::Newline {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push one retained line comment.
    fn push_line_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.tokenizer.file_id());
        let raw_comment = self.tokenizer.span_str(token_span.span);
        let retention = self
            .trivia_mode
            .classify_comment(token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { content, .. } => {
                self.trivia.add_line_comment(token_span, content);
            }
            CommentRetention::Skip => {
                self.record_skipped_comment(token, has_line_terminator);
            }
        }
    }

    /// Push one retained block comment.
    fn push_block_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.tokenizer.file_id());
        let raw_comment = self.tokenizer.span_str(token_span.span);
        let retention = self
            .trivia_mode
            .classify_comment(token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { kind, content } => {
                self.trivia.add_block_comment(token_span, kind, content);
            }
            CommentRetention::Skip => {
                self.record_skipped_comment(token, has_line_terminator);
            }
        }
    }

    /// Record one skipped comment in attachment state.
    fn record_skipped_comment(&mut self, token: Token, has_line_terminator: bool) {
        if has_line_terminator {
            self.trivia.record_newline(token.start());
        } else {
            self.trivia.record_skipped_side_token(token.start());
        }
    }
}
