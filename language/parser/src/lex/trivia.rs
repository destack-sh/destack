use destack_ast::{
    Comment, CommentContent, CommentKind, CommentNewlines, CommentPosition, TokenSpan, TokenType,
};

/// Snapshot of lexer trivia state for speculative lexing.
#[derive(Debug, Copy, Clone)]
pub(super) struct TriviaSnapshot {
    /// The number of comments already collected.
    comments_len: usize,
    /// The number of comments already assigned to a following token.
    processed: usize,
    /// Whether a newline was seen since the last token.
    saw_newline: bool,
    /// Whether a newline was seen since the last token or comment.
    saw_newline_for_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
}

/// Lexer-time raw comment attachment.
#[derive(Debug)]
pub(super) struct Trivia {
    /// The collected comments in source order.
    comments: Vec<Comment>,
    /// The number of comments already assigned to a following token.
    processed: usize,
    /// Whether a newline was seen since the last token.
    saw_newline: bool,
    /// Whether a newline was seen since the last token or comment.
    saw_newline_for_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
}

impl Trivia {
    /// Create one trivia state with newline-leading initial state.
    pub(super) fn new() -> Self {
        Self {
            comments: Vec::new(),
            processed: 0,
            saw_newline: true,
            saw_newline_for_comment: true,
            previous_token_type: TokenType::End,
        }
    }

    /// Return the collected comments.
    pub(super) fn comments(&self) -> &[Comment] {
        &self.comments
    }

    /// Return whether any comments were collected.
    pub(super) fn has_comments(&self) -> bool {
        !self.comments.is_empty()
    }

    /// Snapshot the current trivia state.
    pub(super) fn snapshot(&self) -> TriviaSnapshot {
        TriviaSnapshot {
            comments_len: self.comments.len(),
            processed: self.processed,
            saw_newline: self.saw_newline,
            saw_newline_for_comment: self.saw_newline_for_comment,
            previous_token_type: self.previous_token_type,
        }
    }

    /// Restore trivia state from one snapshot.
    pub(super) fn restore(&mut self, snapshot: TriviaSnapshot) {
        self.comments.truncate(snapshot.comments_len);
        self.processed = snapshot.processed.min(self.comments.len());
        self.saw_newline = snapshot.saw_newline;
        self.saw_newline_for_comment = snapshot.saw_newline_for_comment;
        self.previous_token_type = snapshot.previous_token_type;
    }

    /// Truncate trivia after one byte position and reset the live boundary state.
    pub(super) fn truncate_after(&mut self, boundary_start: u32, previous_token_type: TokenType) {
        self.comments
            .retain(|comment| comment.span.start < boundary_start);
        self.processed = self.processed.min(self.comments.len());
        self.saw_newline = false;
        self.saw_newline_for_comment = false;
        self.previous_token_type = previous_token_type;
    }

    /// Record one line comment.
    pub(super) fn add_line_comment(&mut self, token_span: TokenSpan, source_text: &str) {
        self.add_comment(token_span, CommentKind::Line, source_text);
    }

    /// Record one block comment.
    pub(super) fn add_block_comment(&mut self, token_span: TokenSpan, source_text: &str) {
        // block comment shape
        let kind = if source_text.contains('\n') {
            CommentKind::MultiLineBlock
        } else {
            CommentKind::SingleLineBlock
        };

        self.add_comment(token_span, kind, source_text);
    }

    /// Record one newline boundary after pending comments.
    pub(super) fn handle_newline(&mut self) {
        let comments_len = self.comments.len();

        if self.processed < comments_len {
            if let Some(last_comment) = self.comments.last_mut() {
                last_comment.newlines.bits |= CommentNewlines::TRAILING;
            }

            if !self.saw_newline {
                self.processed = comments_len;
            }
        }

        self.saw_newline = true;
        self.saw_newline_for_comment = true;
    }

    /// Attach pending leading comments to one semantic token boundary.
    pub(super) fn handle_token(&mut self, token_span: TokenSpan) {
        self.previous_token_type = token_span.token.ty;

        if self.processed < self.comments.len() {
            for comment in &mut self.comments[self.processed..] {
                comment.position = CommentPosition::Leading;
                comment.attached_to = token_span.span.start;
            }

            self.processed = self.comments.len();
        }

        self.saw_newline = false;
        self.saw_newline_for_comment = false;
    }

    /// Record one raw comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, _source_text: &str) {
        let mut comment = Comment::new(token_span.span, kind);
        comment.position = CommentPosition::Trailing;
        comment.newlines = CommentNewlines::from_bools(self.saw_newline_for_comment, false);
        comment.content = comment_content(token_span.token.ty);

        // line comments always end the current line
        if kind == CommentKind::Line {
            comment.newlines.bits |= CommentNewlines::TRAILING;

            if self.should_be_trailing_line_comment() {
                self.processed = self.comments.len() + 1;
            }

            self.saw_newline = true;
            self.saw_newline_for_comment = true;
        }
        // block comments only affect the local boundary
        else {
            self.saw_newline_for_comment = false;
        }

        self.comments.push(comment);
    }

    /// Return whether one line comment should stay trailing.
    fn should_be_trailing_line_comment(&self) -> bool {
        !self.saw_newline
            && !matches!(
                self.previous_token_type,
                TokenType::Assign | TokenType::OpenParenthesis
            )
    }
}

/// Return the structured content classification for one raw comment token.
fn comment_content(token_type: TokenType) -> CommentContent {
    match token_type {
        TokenType::DocBlockComment => CommentContent::Jsdoc,
        _ => CommentContent::None,
    }
}
