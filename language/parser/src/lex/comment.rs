use memchr::memchr_iter;
use smallvec::SmallVec;

use tspp_dir::{
    Comment, CommentAnchor, CommentKind, CommentNewlines, CommentRole, TokenSpan, TokenType,
};

use super::CommentRetention;

/// One retained comment awaiting a token anchor.
#[derive(Debug, Copy, Clone)]
struct PendingComment {
    /// The source span of the raw comment, including delimiters.
    span: tspp_source::Span,
    /// The kind of the comment.
    kind: CommentKind,
    /// The newline shape around the comment.
    newlines: CommentNewlines,
    /// The authored role of the comment.
    role: CommentRole,
}

impl PendingComment {
    /// Attach this comment to the semantic token stream.
    fn anchor(self, anchor: CommentAnchor) -> Comment {
        Comment {
            span: self.span,
            anchor,
            kind: self.kind,
            newlines: self.newlines,
            role: self.role,
        }
    }

    /// Record whether a newline follows this comment.
    fn set_followed_by_newline(&mut self, has_newline: bool) {
        self.newlines =
            CommentNewlines::from_bools(self.newlines.has_leading_newline(), has_newline);
    }
}

/// Live lexer comment state.
#[derive(Debug)]
pub(super) struct LexerComments {
    /// The anchored comments in source order.
    comments: Vec<Comment>,
    /// The retained comments awaiting a token anchor.
    pending: SmallVec<[PendingComment; 2]>,
    /// Whether the cursor is after a newline following the previous token.
    has_newline_after_previous_token: bool,
    /// Whether the next comment starts after a newline.
    has_newline_before_next_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
    /// The previous semantic token start.
    previous_token_start: u32,
}

impl LexerComments {
    /// Create one comment buffer with newline-leading initial state.
    pub(super) fn new() -> Self {
        Self {
            comments: Vec::new(),
            pending: SmallVec::new(),
            has_newline_after_previous_token: true,
            has_newline_before_next_comment: true,
            previous_token_type: TokenType::End,
            previous_token_start: 0,
        }
    }

    /// Take the retained comments and leave the comment buffer empty.
    pub(super) fn take_comments(&mut self) -> Vec<Comment> {
        debug_assert!(self.pending.is_empty());

        std::mem::take(&mut self.comments)
    }

    /// Record one line comment.
    pub(super) fn add_line_comment(&mut self, token_span: TokenSpan, role: CommentRole) {
        self.add_comment(token_span, CommentKind::Line, role);
    }

    /// Record one block comment.
    pub(super) fn add_block_comment(
        &mut self,
        token_span: TokenSpan,
        kind: CommentKind,
        role: CommentRole,
    ) {
        self.add_comment(token_span, kind, role);
    }

    /// Record one newline boundary after pending comments.
    pub(super) fn record_newline(&mut self) {
        // mark the final pending comment as line terminated
        if let Some(last_comment) = self.pending.last_mut() {
            last_comment.set_followed_by_newline(true);

            // anchor same line comments after the preceding token
            if self.is_comment_trailing() {
                let anchor = CommentAnchor::After(self.previous_token_start);
                self.anchor_pending(anchor);
            }
        }

        self.has_newline_after_previous_token = true;
        self.has_newline_before_next_comment = true;
    }

    /// Attach pending leading comments to one semantic token start.
    pub(super) fn record_token(&mut self, token_type: TokenType, start: u32) {
        // anchor pending comments before this token or at end of source
        let anchor = if token_type == TokenType::End {
            CommentAnchor::End
        } else {
            CommentAnchor::Before(start)
        };
        self.anchor_pending(anchor);

        self.previous_token_type = token_type;
        self.previous_token_start = start;
        self.has_newline_after_previous_token = false;
        self.has_newline_before_next_comment = false;
    }

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, role: CommentRole) {
        let mut comment = PendingComment {
            span: token_span.span,
            kind,
            newlines: CommentNewlines::from_bools(self.has_newline_before_next_comment, false),
            role,
        };

        // line comments always end the current line
        if kind == CommentKind::Line {
            comment.set_followed_by_newline(true);
            self.pending.push(comment);

            if self.is_comment_trailing() {
                let anchor = CommentAnchor::After(self.previous_token_start);
                self.anchor_pending(anchor);
            }

            self.has_newline_after_previous_token = true;
            self.has_newline_before_next_comment = true;
        }
        // block comments only affect the local boundary
        else {
            self.pending.push(comment);
            self.has_newline_before_next_comment = false;
        }
    }

    /// Attach every pending comment to one token boundary.
    fn anchor_pending(&mut self, anchor: CommentAnchor) {
        self.comments
            .extend(self.pending.drain(..).map(|comment| comment.anchor(anchor)));
    }

    /// Return whether one same-line comment should attach to the previous token.
    fn is_comment_trailing(&self) -> bool {
        !self.has_newline_after_previous_token
            && !matches!(
                self.previous_token_type,
                TokenType::Assign | TokenType::OpenParenthesis
            )
    }
}

/// Retention decision for one comment token.
#[derive(Debug, Copy, Clone)]
pub(super) enum CommentDecision {
    /// Keep the comment with structured metadata.
    Keep {
        /// The line or block comment kind.
        kind: CommentKind,
        /// The authored role of the comment.
        role: CommentRole,
    },
    /// Skip the comment payload.
    Skip,
}

impl CommentRetention {
    /// Classify one comment under this retention mode.
    pub(super) fn classify_comment(
        self,
        token_type: TokenType,
        raw_comment: &str,
    ) -> CommentDecision {
        // discard every comment when retention is disabled
        if self == Self::Ignore {
            return CommentDecision::Skip;
        }

        // retain every comment for formatting
        if self == Self::All {
            let kind = classify_comment_kind(token_type, raw_comment);
            let role = classify_comment_role(token_type, raw_comment);

            return CommentDecision::Keep { kind, role };
        }

        // documentation mode keeps documentation, legal, and preserve comments
        let is_doc_comment = matches!(
            token_type,
            TokenType::DocLineComment | TokenType::DocBlockComment
        );
        let is_legal = is_legal_comment(token_type, raw_comment);
        if !is_doc_comment && !is_legal {
            return CommentDecision::Skip;
        }

        let kind = classify_comment_kind(token_type, raw_comment);
        let role = classify_comment_role(token_type, raw_comment);
        if role == CommentRole::Ordinary {
            return CommentDecision::Skip;
        }

        CommentDecision::Keep { kind, role }
    }
}

/// Return the line or block kind for one comment token.
fn classify_comment_kind(token_type: TokenType, raw_comment: &str) -> CommentKind {
    let is_line = matches!(
        token_type,
        TokenType::LineComment | TokenType::DocLineComment
    );
    debug_assert!(
        is_line
            || matches!(
                token_type,
                TokenType::BlockComment | TokenType::DocBlockComment
            )
    );

    // distinguish line comments from block comment line shapes
    if is_line {
        CommentKind::Line
    } else {
        classify_block_comment(raw_comment)
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

/// Return the authored role of one raw comment token.
pub(super) fn classify_comment_role(token_type: TokenType, raw_comment: &str) -> CommentRole {
    let content = trim_comment_delimiters(token_type, raw_comment);
    let bytes = content.as_bytes();

    if bytes.is_empty() {
        return CommentRole::Ordinary;
    }

    if token_type == TokenType::DocLineComment {
        if contains_legal_marker(content) {
            return CommentRole::LegalDocumentation;
        }

        return CommentRole::Documentation;
    }

    match bytes[0] {
        b'!' => return CommentRole::Legal,
        b'*' if matches!(
            token_type,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) =>
        {
            if bytes.iter().any(|byte| *byte != b'*') {
                if contains_legal_marker(content) {
                    return CommentRole::LegalDocumentation;
                }

                return CommentRole::Documentation;
            }

            return CommentRole::Ordinary;
        }
        _ => {}
    }

    let mut start = 0usize;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    if start >= bytes.len() {
        return CommentRole::Ordinary;
    }

    match bytes[start] {
        b'@' => {
            start += 1;

            if start >= bytes.len() {
                return CommentRole::Ordinary;
            }

            if bytes[start..].starts_with(b"license") || bytes[start..].starts_with(b"preserve") {
                return CommentRole::Legal;
            }
        }

        _ => {
            if contains_legal_marker(content) {
                return CommentRole::Legal;
            }

            return CommentRole::Ordinary;
        }
    }

    if contains_legal_marker(content) {
        return CommentRole::Legal;
    }

    CommentRole::Ordinary
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
