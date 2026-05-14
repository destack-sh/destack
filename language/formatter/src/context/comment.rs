use super::source::SourceText;
use crate::file::comment_text_has_suppression_directive;
use destack_dir::Comment;
use destack_source::Span;

/// One saved comment cursor state for speculative formatting.
#[derive(Debug, Clone, Copy)]
pub struct CommentSnapshot {
    /// The number of comments already printed.
    printed_count: usize,
    /// The optional limit for the visible unprinted comment slice.
    view_limit: Option<usize>,
}

/// Cursor-based access to comments during formatting.
#[derive(Debug, Clone)]
pub struct Comments<'a> {
    /// The comments in source order.
    comments: &'a [Comment],
    /// The source text used for positional comment queries.
    source_text: SourceText<'a>,
    /// The number of comments already printed.
    printed_count: usize,
    /// The optional limit for the visible unprinted comment slice.
    view_limit: Option<usize>,
}

impl<'a> Comments<'a> {
    /// Create one comment cursor over comments.
    pub fn new(source_text: SourceText<'a>, comments: &'a [Comment]) -> Self {
        Self {
            comments,
            source_text,
            printed_count: 0,
            view_limit: None,
        }
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
        let start = self.printed_count.min(self.comments.len());

        // limited speculative formatting can advance the printed cursor past the
        // active view limit, which should yield an empty slice instead of panicking
        let end = self.view_limit.unwrap_or(self.comments.len());
        let end = end.max(start).min(self.comments.len());

        &self.comments[start..end]
    }

    /// Return the printed comments.
    #[inline]
    pub fn printed_comments(&self) -> &'a [Comment] {
        let end = self.printed_count.min(self.comments.len());

        &self.comments[..end]
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
        let start_index = comments.partition_point(|comment| comment.span.end <= pos);
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
        comment_text_has_suppression_directive(self.span_text(comment.content_span()))
    }

    /// Return trailing comments after one preceding span inside one enclosing span.
    pub fn get_trailing_comments(
        &self,
        enclosing_span: Span,
        preceding_span: Span,
        following_span_start: u32,
    ) -> &'a [Comment] {
        // empty
        let comments = self.unprinted_comments();
        if comments.is_empty() {
            return &[];
        }

        let source_text = self.source_text;

        debug_assert!(
            comments
                .first()
                .is_none_or(|comment| comment.span.end > preceding_span.start)
        );

        // no following sibling: everything up to the enclosing end is eligible
        if following_span_start == 0 {
            let comments = self.comments_before(enclosing_span.end);
            let mut start = preceding_span.end;

            for (index, comment) in comments.iter().enumerate() {
                if start > comment.span.start {
                    continue;
                }

                if !source_text.all_bytes_match(start, comment.span.start, |byte| {
                    byte.is_ascii_whitespace() || matches!(byte, b')' | b',' | b';')
                }) {
                    return &comments[..index];
                }

                start = comment.span.end;
            }

            return comments;
        }

        // scan until the following sibling boundary
        let comments = self.comments_after(preceding_span.end);
        let mut comment_index = 0usize;

        while let Some(comment) = comments.get(comment_index) {
            if comment.span.end > following_span_start || comment.span.end > enclosing_span.end {
                break;
            }

            // the following node may sit outside the enclosing span
            if following_span_start > enclosing_span.end && comment.span.end <= enclosing_span.end {
            } else if comment.preceded_by_newline() {
                break;
            } else if comment.followed_by_newline() {
                return &comments[..=comment_index];
            }

            comment_index += 1;
        }

        // walk back to the first comment separated only by whitespace or parens
        let mut gap_end = following_span_start;

        for (index, comment) in comments[..comment_index].iter().enumerate().rev() {
            if source_text.all_bytes_match(comment.span.end, gap_end, |byte| {
                byte.is_ascii_whitespace() || byte == b'('
            }) {
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

    /// Advance the printed cursor past one concrete comment span.
    #[inline]
    pub fn mark_comment_printed(&mut self, comment: Comment) {
        let printed_count = self
            .comments
            .partition_point(|candidate| candidate.span.end <= comment.span.end);
        self.printed_count = self.printed_count.max(printed_count);
    }

    /// Save the current comment cursor state.
    #[inline]
    pub fn snapshot(&self) -> CommentSnapshot {
        CommentSnapshot {
            printed_count: self.printed_count,
            view_limit: self.view_limit,
        }
    }

    /// Restore one saved comment cursor state.
    #[inline]
    pub fn restore(&mut self, snapshot: CommentSnapshot) {
        self.printed_count = snapshot.printed_count;
        self.view_limit = snapshot.view_limit;
    }

    /// Limit the visible unprinted comment slice to one end position.
    pub fn limit_comments_up_to(&mut self, end_pos: u32) -> Option<usize> {
        let original_limit = self.view_limit;
        let limit_index = self.comments[self.printed_count..]
            .iter()
            .position(|comment| comment.span.start >= end_pos)
            .map_or(self.comments.len(), |index| self.printed_count + index);

        if limit_index < self.comments.len() {
            self.view_limit = Some(limit_index);
        }

        original_limit
    }

    /// Restore one previously saved visible comment limit.
    #[inline]
    pub fn restore_view_limit(&mut self, limit: Option<usize>) {
        self.view_limit = limit;
    }

    /// Return the source text for one span.
    fn span_text(&self, span: Span) -> &'a str {
        self.source_text.text_for(&span)
    }
}
