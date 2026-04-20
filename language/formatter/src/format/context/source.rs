use super::comment::Comments;
use super::context::DestackFormatContext;
use std::ops::Deref;

use destack_ast::{Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan};
use destack_source::Span;

/// Source text wrapper for formatter byte and span queries.
#[derive(Debug, Clone, Copy)]
pub struct SourceText<'a> {
    /// The underlying source text.
    text: &'a str,
}

impl Deref for SourceText<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.text
    }
}

impl<'a> SourceText<'a> {
    /// Create one source text wrapper.
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    /// Return the text for one span.
    pub fn text_for(&self, span: &Span) -> &'a str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// Return the slice from one offset.
    pub fn slice_from(&self, position: u32) -> &'a str {
        &self.text[position as usize..]
    }

    /// Return the slice up to one offset.
    pub fn slice_to(&self, position: u32) -> &'a str {
        &self.text[..position as usize]
    }

    /// Return the slice inside one byte range.
    pub fn slice_range(&self, start: u32, end: u32) -> &'a str {
        &self.text[start as usize..end as usize]
    }

    /// Return bytes from one offset to the end.
    pub fn bytes_from(&self, position: u32) -> impl Iterator<Item = u8> + '_ {
        self.text.as_bytes()[position as usize..].iter().copied()
    }

    /// Return bytes before one offset in reverse.
    pub fn bytes_to(&self, position: u32) -> impl Iterator<Item = u8> + '_ {
        self.text.as_bytes()[..position as usize]
            .iter()
            .copied()
            .rev()
    }

    /// Return bytes inside one byte range.
    pub fn bytes_range(&self, start: u32, end: u32) -> &'a [u8] {
        &self.text.as_bytes()[start as usize..end as usize]
    }

    /// Return whether the next non-whitespace byte matches one value.
    pub fn next_non_whitespace_byte_is(&self, position: u32, expected_byte: u8) -> bool {
        self.bytes_from(position)
            .find(|byte| !byte.is_ascii_whitespace())
            .is_some_and(|byte| byte == expected_byte)
    }

    /// Return the byte at one position.
    pub fn byte_at(&self, position: u32) -> Option<u8> {
        self.text.as_bytes().get(position as usize).copied()
    }

    /// Return whether one span contains a newline.
    pub fn contains_newline(&self, span: Span) -> bool {
        self.contains_newline_between(span.start, span.end)
    }

    /// Return whether one byte range contains a newline.
    pub fn contains_newline_between(&self, start: u32, end: u32) -> bool {
        self.slice_range(start, end)
            .bytes()
            .any(|byte| matches!(byte, b'\n' | b'\r'))
    }

    /// Return whether horizontal trivia before one offset contains a newline.
    pub fn has_newline_before(&self, position: u32) -> bool {
        for byte in self.bytes_to(position) {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        false
    }

    /// Return whether horizontal trivia after one offset contains a newline.
    pub fn has_newline_after(&self, position: u32) -> bool {
        for byte in self.bytes_from(position) {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                _ => return false,
            }
        }

        false
    }

    /// Return whether a newline appears after one opening brace, scanning through comments.
    pub fn has_newline_after_opening_brace(&self, position: u32) -> bool {
        let mut iter = self.bytes_from(position + 1).peekable();

        while let Some(byte) = iter.next() {
            match byte {
                b'\n' | b'\r' => return true,
                b' ' | b'\t' => {}
                b'/' => match iter.peek() {
                    Some(&b'/') => {
                        iter.next();
                        return iter.any(|byte| matches!(byte, b'\n' | b'\r'));
                    }
                    Some(&b'*') => {
                        iter.next();

                        while let Some(byte) = iter.next() {
                            if matches!(byte, b'\n' | b'\r') {
                                return true;
                            }

                            if byte == b'*' && matches!(iter.peek(), Some(&b'/')) {
                                iter.next();
                                break;
                            }
                        }
                    }
                    _ => return false,
                },
                _ => return false,
            }
        }

        false
    }

    /// Return whether one byte range contains one byte value.
    pub fn bytes_contain(&self, start: u32, end: u32, byte: u8) -> bool {
        self.bytes_range(start, end).contains(&byte)
    }

    /// Return whether all bytes in one range match one predicate.
    pub fn all_bytes_match<F>(&self, start: u32, end: u32, predicate: F) -> bool
    where
        F: Fn(u8) -> bool,
    {
        self.bytes_range(start, end)
            .iter()
            .all(|byte| predicate(*byte))
    }

    /// Return the character width of one span.
    pub fn span_width(&self, span: Span) -> usize {
        self.text_for(&span).chars().count()
    }

    /// Return the number of consecutive line breaks after one offset.
    pub fn lines_after(&self, end: u32) -> usize {
        let mut count = 0usize;
        let mut chars = self.slice_from(end).chars().peekable();

        while let Some(current) = chars.next() {
            if is_single_line_whitespace(current) {
                continue;
            }

            if is_line_terminator(current) {
                count += 1;

                if current == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }

                continue;
            }

            return count;
        }

        0
    }

    /// Return the number of line breaks before one span.
    pub fn get_lines_before(&self, span: Span, comments: &Comments<'_>) -> usize {
        let mut start = span.start;
        let comments = comments.unprinted_comments();

        // skip leading comments and semicolons that conceptually belong to the node
        if let Some(comment) = comments.first()
            && comment.span.end <= start
        {
            start = comment.span.start;
        } else if start != 0 && matches!(self.byte_at(start - 1), Some(b';')) {
            start -= 1;
        }

        // count line breaks before the node, ignoring balanced wrapper parens
        let mut count = 0usize;
        let mut following_source = self.bytes_from(span.end);
        let mut chars = self.slice_to(start).chars().rev().peekable();

        while let Some(current) = chars.next() {
            if is_single_line_whitespace(current) {
                continue;
            }

            if current == '(' {
                for byte in following_source.by_ref() {
                    if byte.is_ascii_whitespace() {
                        continue;
                    }

                    if byte == b')' {
                        break;
                    }

                    return count;
                }

                count = 0;
                continue;
            }

            if !is_line_terminator(current) {
                return count;
            }

            count += 1;

            if current == '\n' && chars.peek() == Some(&'\r') {
                chars.next();
            }
        }

        0
    }
}

/// Return whether one character is a line terminator.
fn is_line_terminator(current: char) -> bool {
    matches!(current, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Return whether one character is horizontal whitespace.
fn is_single_line_whitespace(current: char) -> bool {
    current.is_whitespace() && !is_line_terminator(current)
}

impl<'a> DestackFormatContext<'a> {
    /// Return the source text wrapper for this file.
    pub fn source_text(&self) -> SourceText<'a> {
        SourceText::new(self.file.text())
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        self.newline_offsets.get_or_init(|| {
            self.file
                .text()
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
                .collect()
        })
    }

    /// Return whether file text contains ignore directive markers.
    pub fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }

    /// Get the source slice backing one span.
    #[inline]
    pub fn span_str(&self, span: Span) -> &'a str {
        self.file.span_str(span)
    }

    /// Get the source slice backing one token span.
    #[inline]
    pub fn token_str(&self, token: TokenSpan) -> &'a str {
        self.span_str(token.span)
    }

    /// Get one source-preserving comment slice.
    #[inline]
    pub fn comment_source_text(&self, comment: Comment) -> &'a str {
        self.span_str(comment.span)
    }

    /// Get one source position for one byte offset.
    #[inline]
    pub fn source_position(&self, offset: u32) -> Option<(u32, u32)> {
        self.file.get_position(offset)
    }

    /// Get the source span for one line index.
    #[inline]
    pub fn source_line_span(&self, line_index: u32) -> Option<Span> {
        self.file.get_line_span(line_index)
    }

    /// Return whether one line prefix contains only whitespace trivia.
    #[inline]
    pub fn line_prefix_is_whitespace(&self, offset: u32) -> bool {
        let Some((line_index, _)) = self.source_position(offset) else {
            return false;
        };
        let Some(line_span) = self.source_line_span(line_index) else {
            return false;
        };

        if line_span.start >= offset {
            return true;
        }

        let prefix_span = Span::new(line_span.file, line_span.start, offset);

        !self.has_non_whitespace_content(prefix_span)
    }

    /// Get the source line distance between two byte offsets.
    #[inline]
    pub fn source_line_distance(&self, start_offset: u32, end_offset: u32) -> Option<u32> {
        let (start_line, _) = self.source_position(start_offset)?;
        let (end_line, _) = self.source_position(end_offset)?;

        end_line.checked_sub(start_line)
    }

    /// Get the raw line prefix string before one byte offset.
    #[inline]
    pub fn line_prefix_text(&self, offset: u32) -> Option<&'a str> {
        let (line_index, column) = self.source_position(offset)?;
        let line_span = self.source_line_span(line_index)?;
        let line_text = self.span_str(line_span);

        line_text.get(..column as usize)
    }

    /// Return whether one node span contains a newline.
    #[inline]
    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.has_newline(self.span(node_id))
    }
}
