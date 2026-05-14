use memchr::memchr_iter;

use destack_dir::{
    Comment, CommentContent, CommentKind, CommentNewlines, CommentPosition, TokenSpan, TokenType,
};

/// Live lexer boundary state for comment attachment.
#[derive(Debug, Copy, Clone)]
struct TriviaState {
    /// The number of comments already assigned to a following token.
    processed: usize,
    /// Whether a newline was seen since the last token.
    saw_newline: bool,
    /// Whether a newline was seen since the last token or comment.
    saw_newline_for_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
}

impl TriviaState {
    /// Create one boundary state with newline-leading initial state.
    fn new() -> Self {
        Self {
            processed: 0,
            saw_newline: true,
            saw_newline_for_comment: true,
            previous_token_type: TokenType::End,
        }
    }
}

/// Live lexer trivia state.
#[derive(Debug)]
pub(super) struct Trivia {
    /// The collected comments in source order.
    comments: Vec<Comment>,
    /// The live boundary state for comment attachment.
    state: TriviaState,
}

impl Trivia {
    /// Create one trivia state with newline-leading initial state.
    pub(super) fn new() -> Self {
        Self {
            comments: Vec::new(),
            state: TriviaState::new(),
        }
    }

    /// Take the collected comments and leave the trivia store empty.
    pub(super) fn take_comments(&mut self) -> Vec<Comment> {
        std::mem::take(&mut self.comments)
    }

    /// Return whether any comments were collected.
    pub(super) fn has_comments(&self) -> bool {
        !self.comments.is_empty()
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
    pub(super) fn handle_newline(&mut self, boundary_start: u32) {
        let active_comment_end = self.active_comment_end(boundary_start);

        if self.state.processed < active_comment_end {
            let last_comment = &mut self.comments[active_comment_end - 1];
            last_comment.newlines.bits |= CommentNewlines::TRAILING;

            if !self.state.saw_newline {
                self.state.processed = active_comment_end;
            }
        }

        self.state.saw_newline = true;
        self.state.saw_newline_for_comment = true;
    }

    /// Attach pending leading comments to one semantic token boundary.
    pub(super) fn handle_token(&mut self, token_span: TokenSpan) {
        self.state.previous_token_type = token_span.token.ty;

        let active_comment_end = self.active_comment_end(token_span.span.start);

        if self.state.processed < active_comment_end {
            for comment in &mut self.comments[self.state.processed..active_comment_end] {
                comment.position = CommentPosition::Leading;
                comment.attached_to = token_span.span.start;
            }

            self.state.processed = active_comment_end;
        }

        self.state.saw_newline = false;
        self.state.saw_newline_for_comment = false;
    }

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, source_text: &str) {
        let mut comment = Comment::new(token_span.span, kind);
        comment.newlines = CommentNewlines::from_bools(self.state.saw_newline_for_comment, false);
        comment.content = comment_content_from_raw(token_span.token.ty, source_text);

        // speculative lexing can revisit the same raw comment span
        let is_duplicate = self
            .comments
            .last()
            .is_some_and(|last_comment| comment.span.start <= last_comment.span.start);

        // line comments always end the current line
        if kind == CommentKind::Line {
            comment.newlines.bits |= CommentNewlines::TRAILING;

            if self.should_attach_comment_to_previous_token() {
                self.state.processed = self.comments.len() + usize::from(!is_duplicate);
            }

            self.state.saw_newline = true;
            self.state.saw_newline_for_comment = true;
        }
        // block comments only affect the local boundary
        else {
            self.state.saw_newline_for_comment = false;
        }

        // keep comment storage monotonic across speculative rewinds
        if !is_duplicate {
            self.comments.push(comment);
        }
    }

    /// Return the exclusive end of comments that are before one token boundary.
    fn active_comment_end(&self, boundary_start: u32) -> usize {
        let mut comment_index = self.state.processed;

        while comment_index < self.comments.len()
            && self.comments[comment_index].span.end <= boundary_start
        {
            comment_index += 1;
        }

        comment_index
    }

    /// Return whether one same-line comment should attach to the previous token.
    fn should_attach_comment_to_previous_token(&self) -> bool {
        !self.state.saw_newline
            && !matches!(
                self.state.previous_token_type,
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
