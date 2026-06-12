use memchr::memchr_iter;

use destack_dir::{
    Comment, CommentContent, CommentKind, CommentNewlines, CommentPosition, TokenSpan, TokenType,
};

use super::ParserTriviaMode;

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
    /// The reversible comment mutations after checkpoints.
    undo: Vec<TriviaUndo>,
}

/// A checkpoint for speculative trivia rollback.
#[derive(Debug, Copy, Clone)]
pub(super) struct TriviaCheckpoint {
    /// The live boundary state.
    state: TriviaState,
    /// The comment count at checkpoint time.
    comments_len: usize,
    /// The undo log length at checkpoint time.
    undo_len: usize,
}

/// One reversible trivia comment mutation.
#[derive(Debug, Copy, Clone)]
struct TriviaUndo {
    /// The comment index.
    index: usize,
    /// The comment value before mutation.
    comment: Comment,
}

impl Trivia {
    /// Create one trivia state with newline-leading initial state.
    pub(super) fn new() -> Self {
        Self {
            comments: Vec::new(),
            state: TriviaState::new(),
            undo: Vec::new(),
        }
    }

    /// Take the collected comments and leave the trivia store empty.
    pub(super) fn take_comments(&mut self) -> Vec<Comment> {
        self.undo.clear();

        std::mem::take(&mut self.comments)
    }

    /// Drain collected comments into one output buffer.
    pub(super) fn drain_comments_into(&mut self, comments: &mut Vec<Comment>) {
        comments.append(&mut self.comments);
        self.undo.clear();
    }

    /// Create a checkpoint for speculative lexer movement.
    pub(super) fn checkpoint(&self) -> TriviaCheckpoint {
        TriviaCheckpoint {
            state: self.state,
            comments_len: self.comments.len(),
            undo_len: self.undo.len(),
        }
    }

    /// Restore a speculative lexer checkpoint.
    pub(super) fn restore(&mut self, checkpoint: TriviaCheckpoint) {
        while self.undo.len() > checkpoint.undo_len {
            let Some(undo) = self.undo.pop() else {
                break;
            };

            if undo.index < checkpoint.comments_len {
                self.comments[undo.index] = undo.comment;
            }
        }

        self.comments.truncate(checkpoint.comments_len);
        self.state = checkpoint.state;
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
    pub(super) fn handle_newline(&mut self, boundary_start: u32) {
        let active_comment_end = self.active_comment_end(boundary_start);

        if self.state.processed < active_comment_end {
            self.record_comment_undo(active_comment_end - 1);
            let last_comment = &mut self.comments[active_comment_end - 1];
            last_comment.newlines.bits |= CommentNewlines::TRAILING;

            if !self.state.saw_newline {
                self.state.processed = active_comment_end;
            }
        }

        self.state.saw_newline = true;
        self.state.saw_newline_for_comment = true;
    }

    /// Record one skipped side-token boundary.
    pub(super) fn handle_skipped_side_token(&mut self, boundary_start: u32) {
        let active_comment_end = self.active_comment_end(boundary_start);

        if self.state.processed < active_comment_end {
            self.record_comment_undo(active_comment_end - 1);
            let last_comment = &mut self.comments[active_comment_end - 1];
            last_comment.newlines.bits |= CommentNewlines::TRAILING;
        }
    }

    /// Attach pending leading comments to one semantic token start.
    pub(super) fn handle_token_start(&mut self, token_type: TokenType, start: u32) {
        self.state.previous_token_type = token_type;

        let active_comment_end = self.active_comment_end(start);

        if self.state.processed < active_comment_end {
            for index in self.state.processed..active_comment_end {
                self.record_comment_undo(index);
                let comment = &mut self.comments[index];
                comment.position = CommentPosition::Leading;
                comment.attached_to = start;
            }

            self.state.processed = active_comment_end;
        }

        self.state.saw_newline = false;
        self.state.saw_newline_for_comment = false;
    }

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, content: CommentContent) {
        let mut comment = Comment::new(token_span.span, kind);
        comment.newlines = CommentNewlines::from_bools(self.state.saw_newline_for_comment, false);
        comment.content = content;

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

    /// Record the previous value before mutating one comment.
    fn record_comment_undo(&mut self, index: usize) {
        self.undo.push(TriviaUndo {
            index,
            comment: self.comments[index],
        });
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

/// Return whether one comment should be kept in a trivia mode.
pub(super) fn retained_comment(
    trivia_mode: ParserTriviaMode,
    token_type: TokenType,
    raw_comment: &str,
) -> CommentRetention {
    // full trivia keeps every comment for formatting
    if trivia_mode == ParserTriviaMode::Full {
        let kind = comment_kind(token_type, raw_comment);
        let content = comment_content_from_raw(token_type, raw_comment);

        return CommentRetention::Keep { kind, content };
    }

    let is_doc_comment = matches!(
        token_type,
        TokenType::DocLineComment | TokenType::DocBlockComment
    );
    let is_legal_comment = ordinary_comment_is_legal(token_type, raw_comment);

    // documentation mode keeps semantic comments only
    if !is_doc_comment && !is_legal_comment {
        return CommentRetention::Skip;
    }

    let kind = comment_kind(token_type, raw_comment);
    let content = comment_content_from_raw(token_type, raw_comment);

    if content == CommentContent::None {
        return CommentRetention::Skip;
    }

    CommentRetention::Keep { kind, content }
}

/// Return the line or block kind for one comment token.
fn comment_kind(token_type: TokenType, raw_comment: &str) -> CommentKind {
    match token_type {
        TokenType::LineComment | TokenType::DocLineComment => CommentKind::Line,
        TokenType::BlockComment | TokenType::DocBlockComment => block_comment_kind(raw_comment),
        _ => CommentKind::Line,
    }
}

/// Return whether an ordinary comment carries legal or preserve semantics.
fn ordinary_comment_is_legal(token_type: TokenType, raw_comment: &str) -> bool {
    if matches!(
        token_type,
        TokenType::DocLineComment | TokenType::DocBlockComment
    ) {
        return false;
    }

    let content = comment_annotation_text(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.first() == Some(&b'!') {
        return true;
    }

    contains_license_or_preserve_comment(content)
}

/// Return the line shape for one raw block comment token.
pub(super) fn block_comment_kind(raw_comment: &str) -> CommentKind {
    if raw_comment.contains('\n') {
        CommentKind::MultiLineBlock
    } else {
        CommentKind::SingleLineBlock
    }
}

/// Return the structured content classification for one raw comment token.
pub(super) fn comment_content_from_raw(token_type: TokenType, raw_comment: &str) -> CommentContent {
    let content = comment_annotation_text(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.is_empty() {
        return CommentContent::None;
    }

    if token_type == TokenType::DocLineComment {
        if contains_license_or_preserve_comment(content) {
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
