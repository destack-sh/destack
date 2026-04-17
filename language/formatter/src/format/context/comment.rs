use super::source::SourceText;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::Comment;
use destack_fir::format::{Format, FormatNode, FormatResult};
use destack_source::{FileId, Span};

const IGNORE_SUPPRESSION_MARKER: &str = "oxfmt-ignore";

/// Return whether one raw comment payload carries the formatter suppression marker.
fn is_ignore_suppression_comment(text: &str) -> bool {
    text.contains(IGNORE_SUPPRESSION_MARKER)
}

/// One saved raw comment cursor state.
#[derive(Debug, Copy, Clone)]
pub struct CommentSnapshot {
    /// The number of comments already printed.
    printed_count: usize,
    /// The optional limit for the visible unprinted comment slice.
    view_limit: Option<usize>,
}

/// Cursor-based access to raw comments during formatting.
#[derive(Debug, Clone)]
pub struct Comments<'a> {
    /// The raw comments in source order.
    inner: &'a [Comment],
    /// The source text used for comment ownership queries.
    source_text: SourceText<'a>,
    /// The number of comments already printed.
    printed_count: usize,
    /// The optional limit for the visible unprinted comment slice.
    view_limit: Option<usize>,
}

impl<'a> Comments<'a> {
    /// Create one comment cursor over raw comments.
    pub fn new(file_id: FileId, source_text: SourceText<'a>, comments: &'a [Comment]) -> Self {
        let _ = file_id;
        Self {
            inner: comments,
            source_text,
            printed_count: 0,
            view_limit: None,
        }
    }

    /// Return one saved raw comment cursor state.
    #[inline]
    pub fn snapshot(&self) -> CommentSnapshot {
        CommentSnapshot {
            printed_count: self.printed_count,
            view_limit: self.view_limit,
        }
    }

    /// Restore one saved raw comment cursor state.
    #[inline]
    pub fn restore(&mut self, snapshot: CommentSnapshot) {
        self.printed_count = snapshot.printed_count;
        self.view_limit = snapshot.view_limit;
    }

    /// Advance the printed cursor past comments ending before one position.
    #[inline]
    pub fn skip_comments_before(&mut self, pos: u32) {
        let count = self.comments_before(pos).len();
        self.printed_count += count;
    }

    /// Return the unprinted comments.
    #[inline]
    pub fn unprinted_comments(&self) -> &'a [Comment] {
        let end = self.view_limit.unwrap_or(self.inner.len());
        let start = self.printed_count.min(end);
        &self.inner[start..end]
    }

    /// Return the printed comments.
    #[inline]
    pub fn printed_comments(&self) -> &'a [Comment] {
        let end = self.view_limit.unwrap_or(self.inner.len());
        let printed_end = self.printed_count.min(end);
        &self.inner[..printed_end]
    }

    /// Return an iterator over comments that end before or at one position.
    #[inline]
    pub fn comments_before_iter(&self, pos: u32) -> impl Iterator<Item = &Comment> {
        self.unprinted_comments()
            .iter()
            .take_while(move |comment| comment.span.end <= pos)
    }

    /// Return all comments that end before or at one position.
    #[inline]
    pub fn comments_before(&self, pos: u32) -> &'a [Comment] {
        let end_index = self.comments_before_iter(pos).count();
        &self.unprinted_comments()[..end_index]
    }

    /// Return all block comments that end before or at one position.
    #[inline]
    pub fn block_comments_before(&self, pos: u32) -> &'a [Comment] {
        let end_index = self
            .comments_before_iter(pos)
            .take_while(|comment| comment.is_block())
            .count();
        &self.unprinted_comments()[..end_index]
    }

    /// Return all line comments that end before or at one position.
    #[inline]
    pub fn line_comments_before(&self, pos: u32) -> &'a [Comment] {
        let end_index = self
            .comments_before_iter(pos)
            .take_while(|comment| comment.is_line())
            .count();
        &self.unprinted_comments()[..end_index]
    }

    /// Return all comments that are on their own line and end before or at one position.
    #[inline]
    pub fn own_line_comments_before(&self, pos: u32) -> &'a [Comment] {
        let end_index = self
            .comments_before_iter(pos)
            .take_while(|comment| comment.preceded_by_newline())
            .count();
        &self.unprinted_comments()[..end_index]
    }

    /// Return all comments that start after one position.
    #[inline]
    pub fn comments_after(&self, pos: u32) -> &'a [Comment] {
        let comments = self.unprinted_comments();
        let start_index = comments.partition_point(|comment| comment.span.end < pos);
        &comments[start_index..]
    }

    /// Return all comments inside one source range.
    #[inline]
    pub fn comments_in_range(&self, start: u32, end: u32) -> &'a [Comment] {
        let comments = self.comments_after(start);
        let end_index = comments.partition_point(|comment| comment.span.end <= end);
        &comments[..end_index]
    }

    /// Return end-of-line comments that follow one position.
    pub fn end_of_line_comments_after(&self, mut pos: u32) -> &'a [Comment] {
        let comments = self.comments_after(pos);

        for (index, comment) in comments.iter().enumerate() {
            if self
                .source_text
                .all_bytes_match(pos, comment.span.start, |byte| {
                    matches!(byte, b'\t' | b' ' | b'=' | b':')
                })
            {
                if comment.is_line() || comment.followed_by_newline() {
                    return &comments[..=index];
                }

                pos = comment.span.end;
            } else {
                break;
            }
        }

        &[]
    }

    /// Return comments that occur before the first instance of one byte value.
    pub fn comments_before_character(&self, mut start: u32, character: u8) -> &'a [Comment] {
        let comments = self.comments_after(start);

        for (index, comment) in comments.iter().enumerate() {
            if self
                .source_text
                .bytes_contain(start, comment.span.start, character)
            {
                return &comments[..index];
            }

            start = comment.span.end;
        }

        comments
    }

    /// Return whether one range contains comments.
    #[inline]
    pub fn has_comment_in_range(&self, start: u32, end: u32) -> bool {
        self.comments_before_iter(end)
            .any(|comment| comment.span.end > start)
    }

    /// Return whether one span contains comments.
    #[inline]
    pub fn has_comment_in_span(&self, span: Span) -> bool {
        self.has_comment_in_range(span.start, span.end)
    }

    /// Return whether comments exist before one position.
    #[inline]
    pub fn has_comment_before(&self, pos: u32) -> bool {
        self.comments_before_iter(pos).next().is_some()
    }

    /// Return whether a leading own-line comment exists before one position.
    #[inline]
    pub fn has_leading_own_line_comment(&self, pos: u32) -> bool {
        self.comments_before_iter(pos)
            .any(|comment| comment.followed_by_newline())
    }

    /// Return whether an end-of-line comment exists after one position.
    #[inline]
    pub fn has_end_of_line_comment_after(&self, pos: u32) -> bool {
        !self.end_of_line_comments_after(pos).is_empty()
    }

    /// Return whether one node start has a suppression comment.
    pub fn is_suppressed(&self, pos: u32) -> bool {
        self.comments_before(pos)
            .iter()
            .any(|comment| self.is_suppression_comment(comment))
    }

    /// Return whether one comment is a suppression comment.
    pub fn is_suppression_comment(&self, comment: &Comment) -> bool {
        is_ignore_suppression_comment(self.span_text(comment.content_span()))
    }

    /// Return trailing comments owned by one preceding span inside one enclosing span.
    pub fn get_trailing_comments(
        &self,
        enclosing_span: Span,
        preceding_span: Span,
        boundary_start: u32,
        following_span_start: u32,
    ) -> &'a [Comment] {
        let comments = self.comments_after(preceding_span.start);
        if comments.is_empty() {
            return &[];
        }

        if following_span_start == 0 {
            let end_index =
                comments.partition_point(|comment| comment.span.end <= enclosing_span.end);
            let comments = &comments[..end_index];
            let mut start = preceding_span.end;

            for (index, comment) in comments.iter().enumerate() {
                if start > comment.span.start {
                    continue;
                }

                if !self
                    .source_text
                    .all_bytes_match(start, comment.span.start, |byte| {
                        byte.is_ascii_whitespace() || matches!(byte, b')' | b',' | b';')
                    })
                {
                    return &comments[..index];
                }

                start = comment.span.end;
            }

            return comments;
        }

        let trailing_boundary_start = boundary_start.min(following_span_start);

        let mut comment_index = 0usize;

        while let Some(comment) = comments.get(comment_index) {
            if comment.span.end > trailing_boundary_start || comment.span.end > enclosing_span.end {
                break;
            }

            if following_span_start > enclosing_span.end && comment.span.end <= enclosing_span.end {
                // keep scanning
            } else if comment.preceded_by_newline() {
                break;
            } else if comment.followed_by_newline() {
                return &comments[..=comment_index];
            }

            comment_index += 1;
        }

        let mut gap_end = trailing_boundary_start;

        for (index, comment) in comments[..comment_index].iter().enumerate().rev() {
            if self
                .source_text
                .all_bytes_match(comment.span.end, gap_end, |byte| {
                    byte.is_ascii_whitespace() || byte == b'('
                })
            {
                gap_end = comment.span.start;
            } else {
                return &comments[..=index];
            }
        }

        &[]
    }

    /// Advance the printed cursor by one comment.
    #[inline]
    pub fn increment_printed_count(&mut self) {
        self.printed_count += 1;
    }

    /// Advance the printed cursor by several comments.
    #[inline]
    pub fn increase_printed_count_by(&mut self, count: usize) {
        self.printed_count += count;
    }

    /// Advance the printed cursor by one expected comment.
    #[inline]
    pub fn consume(&mut self, comment: Comment) {
        let _ = comment;
        self.increment_printed_count();
    }

    /// Limit the visible unprinted comment slice to one end position.
    pub fn limit_comments_up_to(&mut self, end_pos: u32) -> Option<usize> {
        let original_limit = self.view_limit;
        let limit_index = self.inner[self.printed_count..]
            .iter()
            .position(|comment| comment.span.start >= end_pos)
            .map_or(self.inner.len(), |index| self.printed_count + index);

        if limit_index < self.inner.len() {
            self.view_limit = Some(limit_index);
        }

        original_limit
    }

    /// Restore one previously saved visible comment limit.
    #[inline]
    pub fn restore_view_limit(&mut self, limit: Option<usize>) {
        self.view_limit = limit;
    }

    /// Return the raw source text for one span.
    fn span_text(&self, span: Span) -> &'a str {
        self.source_text.text_for(&span)
    }
}

/// Comment-safe speculative formatting helpers for the local formatter type.
pub(crate) trait DestackFormatterCommentExt<'ast> {
    /// Intern one formatting fragment without mutating the live raw comment cursor.
    fn intern_with_comment_snapshot(
        &mut self,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<Option<FormatNode>>;

    /// Intern one formatting fragment after skipping comments before one offset.
    fn intern_with_comment_snapshot_after(
        &mut self,
        start_offset: Option<u32>,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<Option<FormatNode>>;
}

impl<'ast> DestackFormatterCommentExt<'ast> for DestackFormatter<'ast, '_> {
    /// Intern one formatting fragment without mutating the live raw comment cursor.
    fn intern_with_comment_snapshot(
        &mut self,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<Option<FormatNode>> {
        self.intern_with_comment_snapshot_after(None, content)
    }

    /// Intern one formatting fragment after skipping comments before one offset.
    fn intern_with_comment_snapshot_after(
        &mut self,
        start_offset: Option<u32>,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<Option<FormatNode>> {
        let snapshot = {
            let comments = self.context().comments();
            comments.snapshot()
        };

        if let Some(start_offset) = start_offset {
            self.context()
                .comments_mut()
                .skip_comments_before(start_offset);
        }

        let result = self.intern(content);
        self.context().comments_mut().restore(snapshot);
        result
    }
}
