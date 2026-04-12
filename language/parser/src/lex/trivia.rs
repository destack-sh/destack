use destack_ast::{CommentContent, CommentKind, CommentNewlines, TokenSpan, TokenType};
use destack_source::{NodeSourceMap, SourcePartKey};

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
    comments: Vec<TriviaComment>,
    /// The number of comments already assigned to a following token.
    processed: usize,
    /// Whether a newline was seen since the last token.
    saw_newline: bool,
    /// Whether a newline was seen since the last token or comment.
    saw_newline_for_comment: bool,
    /// The previous non-newline semantic token type.
    previous_token_type: TokenType,
}

/// Comment attachment direction before parser ownership resolution.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TriviaCommentPlacement {
    /// The comment belongs to the following structural owner.
    Leading,
    /// The comment belongs to the previous structural owner.
    Trailing,
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
    pub(super) fn comments(&self) -> &[TriviaComment] {
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
                comment.placement = TriviaCommentPlacement::Leading;
            }

            self.processed = self.comments.len();
        }

        self.saw_newline = false;
        self.saw_newline_for_comment = false;
    }

    /// Record one comment and classify its token-local attachment.
    fn add_comment(&mut self, token_span: TokenSpan, kind: CommentKind, _source_text: &str) {
        let mut comment = TriviaComment {
            span: token_span.span,
            kind,
            placement: TriviaCommentPlacement::Trailing,
            newlines: CommentNewlines::default(),
            content: CommentContent::None,
        };
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

/// One lexer comment before structural parser attachment.
#[derive(Debug, Copy, Clone)]
pub(crate) struct TriviaComment {
    /// The span of the comment, including delimiters.
    pub span: destack_source::Span,
    /// The kind of the comment.
    pub kind: CommentKind,
    /// The structural owner direction.
    placement: TriviaCommentPlacement,
    /// The newline shape around the comment.
    pub newlines: CommentNewlines,
    /// The structured comment content classification.
    pub content: CommentContent,
}

impl TriviaComment {
    /// Return the structural source owner for this comment.
    pub(crate) fn attached_part(&self, source_map: &NodeSourceMap) -> Option<SourcePartKey> {
        let enclosing_owner = source_map.find_innermost_enclosing_owner(self.span);
        let directional_owner = match self.placement {
            TriviaCommentPlacement::Leading => {
                source_map.find_nearest_enclosing_owner_after(self.span.file, self.span.end)
            }
            TriviaCommentPlacement::Trailing => {
                source_map.find_nearest_enclosing_owner_before(self.span.file, self.span.start)
            }
        };

        enclosing_owner.or(directional_owner)
    }
}

/// Return the structured content classification for one comment token.
fn comment_content(token_type: TokenType) -> CommentContent {
    match token_type {
        TokenType::DocBlockComment => CommentContent::Jsdoc,
        _ => CommentContent::None,
    }
}
