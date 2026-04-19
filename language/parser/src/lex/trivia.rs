use memchr::memchr_iter;

use destack_ast::{
    Comment, CommentContent, CommentKind, CommentNewlines, CommentPosition, TokenSpan, TokenType,
};

/// Restore mark for lexer trivia during speculative lexing.
#[derive(Debug, Copy, Clone)]
pub(super) struct TriviaMark {
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

/// Live lexer trivia state.
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

    /// Capture one restore mark for the current trivia state.
    pub(super) fn mark(&self) -> TriviaMark {
        TriviaMark {
            comments_len: self.comments.len(),
            processed: self.processed,
            saw_newline: self.saw_newline,
            saw_newline_for_comment: self.saw_newline_for_comment,
            previous_token_type: self.previous_token_type,
        }
    }

    /// Restore trivia state from one previously captured mark.
    pub(super) fn restore(&mut self, mark: TriviaMark) {
        self.comments.truncate(mark.comments_len);
        self.processed = mark.processed.min(self.comments.len());
        self.saw_newline = mark.saw_newline;
        self.saw_newline_for_comment = mark.saw_newline_for_comment;
        self.previous_token_type = mark.previous_token_type;
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

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, source_text: &str) {
        let mut comment = Comment::new(token_span.span, kind);
        comment.newlines = CommentNewlines::from_bools(self.saw_newline_for_comment, false);
        comment.content = comment_content_from_raw(token_span.token.ty, source_text);

        // line comments always end the current line
        if kind == CommentKind::Line {
            comment.newlines.bits |= CommentNewlines::TRAILING;

            if self.should_attach_comment_to_previous_token() {
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

    /// Return whether one same-line comment should attach to the previous token.
    fn should_attach_comment_to_previous_token(&self) -> bool {
        !self.saw_newline
            && !matches!(
                self.previous_token_type,
                TokenType::Assign | TokenType::OpenParenthesis
            )
    }
}

/// One lexer comment retained in source order.
pub(crate) type TriviaComment = Comment;

/// Return the structured content classification for one raw comment token.
fn comment_content_from_raw(token_type: TokenType, raw_comment: &str) -> CommentContent {
    let content = comment_annotation_text(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.is_empty() {
        return CommentContent::None;
    }

    match bytes[0] {
        b'!' => return CommentContent::Legal,
        b'*' if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) =>
        {
            if bytes.iter().any(|byte| *byte != b'*') {
                if contains_license_or_preserve_comment(content) {
                    return CommentContent::JsdocLegal;
                }

                return CommentContent::Jsdoc;
            }

            return CommentContent::None;
        }
        _ => {}
    }

    let mut start = 0usize;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    if start >= bytes.len() {
        return CommentContent::None;
    }

    match bytes[start] {
        b'@' => {
            start += 1;

            if start >= bytes.len() {
                return CommentContent::None;
            }

            if bytes[start..].starts_with(b"license") || bytes[start..].starts_with(b"preserve") {
                return CommentContent::Legal;
            }
        }

        _ => {
            if contains_license_or_preserve_comment(content) {
                return CommentContent::Legal;
            }

            return CommentContent::None;
        }
    }

    if contains_license_or_preserve_comment(content) {
        return CommentContent::Legal;
    }

    CommentContent::None
}

/// Return the annotation body used for comment classification.
fn comment_annotation_text(token_type: TokenType, raw_comment: &str) -> &str {
    match token_type {
        TokenType::LineComment | TokenType::DocLineComment => raw_comment.strip_prefix("//"),
        TokenType::BlockComment | TokenType::DocBlockComment => raw_comment
            .strip_prefix("/*")
            .and_then(|raw_comment| raw_comment.strip_suffix("*/")),
        _ => None,
    }
    .unwrap_or(raw_comment)
}

/// Return whether a comment contains one legal or preserve marker.
fn contains_license_or_preserve_comment(comment: &str) -> bool {
    let bytes = comment.as_bytes();

    if bytes.len() < 9 {
        return false;
    }

    let search_len = bytes.len() - 8;

    for index in memchr_iter(b'@', &bytes[..search_len]) {
        match bytes[index + 1] {
            b'l' if bytes[index + 2..index + 8] == *b"icense" => return true,
            b'p' if bytes[index + 2..index + 9] == *b"reserve" => return true,
            _ => {}
        }
    }

    false
}
