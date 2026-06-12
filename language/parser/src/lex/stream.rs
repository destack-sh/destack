use std::sync::Arc;

use destack_dir::{Comment, Token, TokenSpan, TokenType};
use destack_source::{File, LanguageType};

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
    pub(crate) fn next_semantic_token(&mut self) -> Token {
        if self.trivia_mode.keeps_side_tokens() {
            return self.next_full_semantic_token();
        }

        loop {
            if self.is_finished {
                return self
                    .eof_token
                    .unwrap_or_else(|| Token::eof(self.position() as u32));
            }

            if let Some(token) = self.skip_trivia_before_semantic_token() {
                return token;
            }

            let token = self.read_source_token();

            if token.is_semantic() {
                return self.finish_semantic_token(token);
            }

            let has_line_terminator = self.side_token_had_line_terminator();
            self.handle_side_token_trivia(token, has_line_terminator);
        }
    }

    /// Return one semantic token while preserving side token buffers.
    fn next_full_semantic_token(&mut self) -> Token {
        loop {
            if self.is_finished {
                return self
                    .eof_token
                    .unwrap_or_else(|| Token::eof(self.position() as u32));
            }

            let token = self.read_source_token();

            if token.is_semantic() {
                return self.finish_semantic_token(token);
            }

            let has_line_terminator = self.side_token_had_line_terminator();
            self.push_side_token(token, has_line_terminator);
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
        let mut lexer = Lexer::new_storing_tokens(file, language);
        lexer.set_trivia_mode(ParserTriviaMode::Full);
        lexer.lex_to_result()
    }

    /// Lex the input string with trivia retention configured.
    pub fn lex_with_options(
        file: Arc<File>,
        language: LanguageType,
        trivia_mode: ParserTriviaMode,
    ) -> LexResult {
        let mut lexer = Lexer::new_storing_tokens(file, language);
        lexer.set_trivia_mode(trivia_mode);
        lexer.lex_to_result()
    }

    /// Finish lexing and extract the stream buffers.
    fn lex_to_result(&mut self) -> LexResult {
        let eof_token = self.eof_token_span();
        let comments = self.take_trivia_comments();
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
    pub(crate) fn take_trivia_comments(&mut self) -> Vec<TriviaComment> {
        self.trivia.take_comments()
    }

    /// Drain trivia comments collected during lexing into one output buffer.
    #[inline]
    pub(crate) fn drain_trivia_comments_into(&mut self, comments: &mut Vec<TriviaComment>) {
        self.trivia.drain_comments_into(comments);
    }

    /// Get the EOF token span, lexing until the end if needed.
    pub fn eof_token_span(&mut self) -> TokenSpan {
        TokenSpan::new(self.eof_token(), self.file_id())
    }

    /// Get the EOF token, lexing until the end if needed.
    pub fn eof_token(&mut self) -> Token {
        // lex until EOF is reached
        while !self.is_finished {
            self.lex_next();
        }

        debug_assert!(
            self.eof_token.is_some(),
            "lexer must produce an eof token after lexing to end"
        );

        self.eof_token
            .unwrap_or_else(|| Token::eof(self.position() as u32))
    }

    /// Ensure all tokens are lexed.
    pub fn lex_to_end(&mut self) {
        while !self.is_finished {
            self.lex_next();
        }
    }

    /// Return owned token span buffers and leave the stream empty.
    pub fn take_token_spans(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        let file_id = self.file_id();
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

    /// Drain side tokens produced so far into one compact output buffer.
    #[inline]
    pub(crate) fn drain_side_tokens_into(&mut self, side_tokens: &mut Vec<Token>) {
        side_tokens.append(&mut self.side_tokens);
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
        let token = self.read_source_token();

        if token.is_semantic() {
            self.push_semantic_token(token);
        } else {
            let has_line_terminator = self.side_token_had_line_terminator();
            self.push_side_token(token, has_line_terminator);
        }

        if token.is(TokenType::End) {
            self.is_finished = true;
            self.eof_token = Some(token);
        }
    }

    /// Push a side token and update stream flags that depend on side tokens.
    #[inline]
    pub(super) fn push_side_token(&mut self, token: Token, has_line_terminator: bool) {
        self.handle_side_token_trivia(token, has_line_terminator);

        if self.trivia_mode.keeps_side_tokens() {
            self.side_tokens.push(token);
        }
    }

    /// Update parser-visible trivia state for one side token.
    #[inline]
    fn handle_side_token_trivia(&mut self, token: Token, has_line_terminator: bool) {
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
                    self.trivia.handle_newline(token.start());
                }
                _ => {}
            }
        }

        if has_line_terminator || token_type == TokenType::Newline {
            self.pending_line_terminator_before_next = true;
        }
    }

    /// Push a semantic token and update stream state.
    pub(super) fn push_semantic_token(&mut self, mut token: Token) {
        token = self.prepare_semantic_token(token);
        self.tokens.push(token);
    }

    /// Prepare one semantic token for parser consumption.
    pub(super) fn prepare_semantic_token(&mut self, mut token: Token) -> Token {
        token = token.with_on_new_line(self.pending_line_terminator_before_next);
        self.pending_line_terminator_before_next = false;

        if self.trivia_mode.keeps_comments() {
            self.trivia.handle_token_start(token.ty(), token.start());
        }

        token
    }

    /// Finish one semantic token read from the lexer cursor.
    #[inline]
    fn finish_semantic_token(&mut self, token: Token) -> Token {
        let token = self.prepare_semantic_token(token);

        if token.is(TokenType::End) {
            self.is_finished = true;
            self.eof_token = Some(token);
        }

        token
    }

    /// Skip ordinary trivia without materializing side tokens.
    fn skip_trivia_before_semantic_token(&mut self) -> Option<Token> {
        loop {
            let start = self.position() as u32;
            let byte = self.scanner.byte();

            match byte {
                b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C => {
                    self.skip_ascii_trivia_run(start);
                    self.reset_token_start();
                }
                b'/' if self.scanner.byte_at(1) == b'/' => {
                    self.skip_line_comment_trivia(start);
                    self.reset_token_start();
                }
                b'/' if self.scanner.byte_at(1) == b'*' => {
                    if let Some(token_range) = self.skip_block_comment_trivia(start) {
                        return Some(token_range);
                    }
                    self.reset_token_start();
                }
                _ => return None,
            }
        }
    }

    /// Skip one contiguous run of ordinary ASCII trivia.
    #[inline]
    fn skip_ascii_trivia_run(&mut self, start: u32) {
        let bytes = self.scanner.remaining_bytes();
        let mut count = 0usize;
        let mut first_newline_start = None;
        let mut last_byte = 0;

        while count < bytes.len() {
            let byte = bytes[count];

            // ordinary spaces
            if matches!(byte, b' ' | b'\t' | 0x0B | 0x0C) {
                last_byte = byte;
                count += 1;
            }
            // line feed
            else if byte == b'\n' {
                first_newline_start.get_or_insert(start + count as u32);
                last_byte = byte;
                count += 1;
            }
            // carriage return, with optional line feed
            else if byte == b'\r' {
                first_newline_start.get_or_insert(start + count as u32);
                last_byte = byte;
                count += 1;
                if bytes.get(count).copied() == Some(b'\n') {
                    last_byte = b'\n';
                    count += 1;
                }
            } else {
                break;
            }
        }

        if count == 0 {
            return;
        }

        self.scanner.advance_ascii_bytes(count, last_byte);

        if let Some(newline_start) = first_newline_start {
            self.handle_compact_newline(newline_start);
        }
    }

    /// Record a skipped newline boundary.
    #[inline]
    fn handle_compact_newline(&mut self, start: u32) {
        if self.trivia_mode.keeps_comments() {
            self.trivia.handle_newline(start);
        }

        self.pending_line_terminator_before_next = true;
    }

    /// Skip one line comment without storing a side token.
    fn skip_line_comment_trivia(&mut self, start: u32) {
        let bytes = self.scanner.remaining_bytes();
        let is_doc_line =
            bytes.get(2).copied() == Some(b'/') && bytes.get(3).copied() != Some(b'/');
        let token_type = if is_doc_line {
            TokenType::DocLineComment
        } else {
            TokenType::LineComment
        };

        self.scanner.advance_ascii_bytes(2, b'/');
        self.eat_until(b'\n');
        self.handle_comment_trivia(token_type, start, true);
    }

    /// Skip one block comment without storing a side token.
    fn skip_block_comment_trivia(&mut self, start: u32) -> Option<Token> {
        let bytes = self.scanner.remaining_bytes();
        let is_doc_block =
            bytes.get(2).copied() == Some(b'*') && bytes.get(3).copied() != Some(b'*');
        let token_type = if is_doc_block {
            TokenType::DocBlockComment
        } else {
            TokenType::BlockComment
        };

        self.scanner.advance_ascii_bytes(2, b'*');
        let (is_terminated, has_line_terminator) = self.eat_block_comment();
        if is_terminated {
            self.handle_comment_trivia(token_type, start, has_line_terminator);
            return None;
        }

        let len = self.position() as u32 - start;
        let token = Token::simple(TokenType::Unknown, start, len);
        self.reset_token_start();

        Some(self.finish_semantic_token(token))
    }

    /// Record one skipped comment when the active trivia mode keeps it.
    fn handle_comment_trivia(
        &mut self,
        token_type: TokenType,
        start: u32,
        has_line_terminator: bool,
    ) {
        let len = self.position() as u32 - start;
        let token = Token::simple(token_type, start, len);

        self.handle_side_token_trivia(token, has_line_terminator);
    }

    /// Push one retained line comment.
    fn push_line_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.file_id());
        let raw_comment = self.get_span_str(token_span.span);
        let retention = retained_comment(self.trivia_mode, token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { content, .. } => {
                self.trivia.add_line_comment(token_span, content);
            }
            CommentRetention::Skip => {
                self.push_skipped_comment_boundary(token, has_line_terminator);
            }
        }
    }

    /// Push one retained block comment.
    fn push_block_comment(&mut self, token: Token, has_line_terminator: bool) {
        let token_span = TokenSpan::new(token, self.file_id());
        let raw_comment = self.get_span_str(token_span.span);
        let retention = retained_comment(self.trivia_mode, token_span.token.ty(), raw_comment);

        match retention {
            CommentRetention::Keep { kind, content } => {
                self.trivia.add_block_comment(token_span, kind, content);
            }
            CommentRetention::Skip => {
                self.push_skipped_comment_boundary(token, has_line_terminator);
            }
        }
    }

    /// Push one skipped comment boundary.
    fn push_skipped_comment_boundary(&mut self, token: Token, has_line_terminator: bool) {
        if has_line_terminator {
            self.trivia.handle_newline(token.start());
        } else {
            self.trivia.handle_skipped_side_token(token.start());
        }
    }
}
