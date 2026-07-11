use memchr::memchr_iter;

use destack_dir::{
    Comment, CommentContent, CommentKind, CommentNewlines, CommentPosition, TokenSpan, TokenType,
};

use super::ParserTriviaMode;

/// Live lexer state for comment attachment.
#[derive(Debug, Copy, Clone)]
struct TriviaState {
    /// The number of comments already assigned to a following token.
    processed: usize,
    /// Whether the cursor is after a newline following the previous token.
    is_after_token_newline: bool,
    /// Whether the next comment starts after a newline.
    has_newline_before_next_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
}

impl TriviaState {
    /// Create attachment state at the beginning of a file.
    fn new() -> Self {
        Self {
            processed: 0,
            is_after_token_newline: true,
            has_newline_before_next_comment: true,
            previous_token_type: TokenType::End,
        }
    }
}

/// Live lexer trivia state.
#[derive(Debug)]
pub(super) struct Trivia {
    /// The collected comments in source order.
    comments: Vec<Comment>,
    /// The live state for comment attachment.
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

    /// Record one line comment.
    pub(super) fn add_line_comment(&mut self, token_span: TokenSpan, content: CommentContent) {
        self.add_comment(token_span, CommentKind::Line, content);
    }

    /// Record one block comment.
    pub(super) fn add_block_comment(
        &mut self,
        token_span: TokenSpan,
        kind: CommentKind,
        content: CommentContent,
    ) {
        self.add_comment(token_span, kind, content);
    }

    /// Record one newline boundary after pending comments.
    pub(super) fn record_newline(&mut self, boundary_start: u32) {
        let active_comment_end = self.active_comment_end(boundary_start);

        if self.state.processed < active_comment_end {
            let last_comment = &mut self.comments[active_comment_end - 1];
            last_comment.newlines.bits |= CommentNewlines::TRAILING;

            if !self.state.is_after_token_newline {
                self.state.processed = active_comment_end;
            }
        }

        self.state.is_after_token_newline = true;
        self.state.has_newline_before_next_comment = true;
    }

    /// Record one skipped side-token boundary.
    pub(super) fn record_skipped_side_token(&mut self, boundary_start: u32) {
        let active_comment_end = self.active_comment_end(boundary_start);

        if self.state.processed < active_comment_end {
            let last_comment = &mut self.comments[active_comment_end - 1];
            last_comment.newlines.bits |= CommentNewlines::TRAILING;
        }
    }

    /// Attach pending leading comments to one semantic token start.
    pub(super) fn record_token(&mut self, token_type: TokenType, start: u32) {
        self.state.previous_token_type = token_type;

        let active_comment_end = self.active_comment_end(start);

        if self.state.processed < active_comment_end {
            for index in self.state.processed..active_comment_end {
                let comment = &mut self.comments[index];
                comment.position = CommentPosition::Leading;
                comment.attached_to = start;
            }

            self.state.processed = active_comment_end;
        }

        self.state.is_after_token_newline = false;
        self.state.has_newline_before_next_comment = false;
    }

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, content: CommentContent) {
        let mut comment = Comment::new(token_span.span, kind);
        comment.newlines =
            CommentNewlines::from_bools(self.state.has_newline_before_next_comment, false);
        comment.content = content;

        // line comments always end the current line
        if kind == CommentKind::Line {
            comment.newlines.bits |= CommentNewlines::TRAILING;

            if self.should_attach_comment_to_previous_token() {
                self.state.processed = self.comments.len() + 1;
            }

            self.state.is_after_token_newline = true;
            self.state.has_newline_before_next_comment = true;
        }
        // block comments only affect the local boundary
        else {
            self.state.has_newline_before_next_comment = false;
        }

        self.comments.push(comment);
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
        !self.state.is_after_token_newline
            && !matches!(
                self.state.previous_token_type,
                TokenType::Assign | TokenType::OpenParenthesis
            )
    }
}

/// Retention decision for one comment token.
#[derive(Debug, Copy, Clone)]
pub(super) enum CommentRetention {
    /// Keep the comment with structured metadata.
    Keep {
        /// The line or block comment kind.
        kind: CommentKind,
        /// The structured comment content classification.
        content: CommentContent,
    },
    /// Skip the comment payload.
    Skip,
}

impl ParserTriviaMode {
    /// Classify one comment under this retention mode.
    pub(super) fn classify_comment(
        self,
        token_type: TokenType,
        raw_comment: &str,
    ) -> CommentRetention {
        // full trivia keeps every comment for formatting
        if self == Self::Full {
            let kind = classify_comment_kind(token_type, raw_comment);
            let content = decode_comment_content(token_type, raw_comment);

            return CommentRetention::Keep { kind, content };
        }

        // documentation mode keeps documentation, legal, and preserve comments
        let is_doc_comment = matches!(
            token_type,
            TokenType::DocLineComment | TokenType::DocBlockComment
        );
        let is_legal = is_legal_comment(token_type, raw_comment);
        if !is_doc_comment && !is_legal {
            return CommentRetention::Skip;
        }

        let kind = classify_comment_kind(token_type, raw_comment);
        let content = decode_comment_content(token_type, raw_comment);
        if content == CommentContent::None {
            return CommentRetention::Skip;
        }

        CommentRetention::Keep { kind, content }
    }
}

/// Return the line or block kind for one comment token.
fn classify_comment_kind(token_type: TokenType, raw_comment: &str) -> CommentKind {
    match token_type {
        TokenType::LineComment | TokenType::DocLineComment => CommentKind::Line,
        TokenType::BlockComment | TokenType::DocBlockComment => classify_block_comment(raw_comment),
        _ => CommentKind::Line,
    }
}

/// Return whether an ordinary comment carries legal or preserve semantics.
fn is_legal_comment(token_type: TokenType, raw_comment: &str) -> bool {
    if matches!(
        token_type,
        TokenType::DocLineComment | TokenType::DocBlockComment
    ) {
        return false;
    }

    let content = trim_comment_delimiters(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.first() == Some(&b'!') {
        return true;
    }

    contains_legal_marker(content)
}

/// Return the line shape for one raw block comment token.
pub(super) fn classify_block_comment(raw_comment: &str) -> CommentKind {
    if raw_comment.contains('\n') {
        CommentKind::MultiLineBlock
    } else {
        CommentKind::SingleLineBlock
    }
}

/// Return the structured content classification for one raw comment token.
pub(super) fn decode_comment_content(token_type: TokenType, raw_comment: &str) -> CommentContent {
    let content = trim_comment_delimiters(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.is_empty() {
        return CommentContent::None;
    }

    if token_type == TokenType::DocLineComment {
        if contains_legal_marker(content) {
            return CommentContent::JsdocLegal;
        }

        return CommentContent::Jsdoc;
    }

    match bytes[0] {
        b'!' => return CommentContent::Legal,
        b'*' if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) =>
        {
            if bytes.iter().any(|byte| *byte != b'*') {
                if contains_legal_marker(content) {
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
            if contains_legal_marker(content) {
                return CommentContent::Legal;
            }

            return CommentContent::None;
        }
    }

    if contains_legal_marker(content) {
        return CommentContent::Legal;
    }

    CommentContent::None
}

/// Return the annotation body used for comment classification.
fn trim_comment_delimiters(token_type: TokenType, raw_comment: &str) -> &str {
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
fn contains_legal_marker(comment: &str) -> bool {
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
