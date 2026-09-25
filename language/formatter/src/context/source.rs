use super::comment::Comments;
use super::context::TsppFormatContext;
use std::ops::Deref;

use tspp_dir::{LocalNodeId, Node, TokenSpan, Tree, TreeStore};
use tspp_source::Span;

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
pub(crate) fn is_line_terminator(current: char) -> bool {
    matches!(current, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Return whether one character is horizontal whitespace.
fn is_single_line_whitespace(current: char) -> bool {
    current.is_whitespace() && !is_line_terminator(current)
}

impl<'a> TsppFormatContext<'a> {
    /// Return the source text wrapper for this file.
    pub fn source_text(&self) -> SourceText<'a> {
        SourceText::new(self.file.text())
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        self.source_index.newline_offsets()
    }

    /// Return whether file text contains ignore directive markers.
    pub fn has_ignore_directive_markers(&self) -> bool {
        self.source_index.has_ignore_directive_markers()
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

    /// Return whether one node span contains a newline.
    #[inline]
    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.has_newline(self.span(node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tspp_source::FileId;

    // line endings

    /// Keep line counting stable for LF input.
    #[test]
    fn test_source_text_counts_lines_before_for_lf_input() {
        let source_text = r"
const x = 1;

const y = 2;


const z = 3;
"
        .trim();
        let source_text = SourceText::new(source_text);
        let comments = vec![];
        let comments = Comments::new(source_text, &comments);
        let file_id = FileId::from_logical_str("formatter/source-lf.tspp");

        let span_x = Span::new(file_id, 0, 12);
        let span_y = Span::new(file_id, 14, 26);
        let span_z = Span::new(file_id, 29, 41);

        assert_eq!(source_text.text_for(&span_x), "const x = 1;");
        assert_eq!(source_text.text_for(&span_y), "const y = 2;");
        assert_eq!(source_text.text_for(&span_z), "const z = 3;");

        assert_eq!(source_text.get_lines_before(span_x, &comments), 0);
        assert_eq!(source_text.get_lines_before(span_y, &comments), 2);
        assert_eq!(source_text.get_lines_before(span_z, &comments), 3);

        assert_eq!(source_text.lines_after(span_x.end), 2);
        assert_eq!(source_text.lines_after(span_y.end), 3);
        assert_eq!(source_text.lines_after(span_z.end), 0);
    }

    /// Keep line counting stable for CRLF input.
    #[test]
    fn test_source_text_counts_lines_before_for_crlf_input() {
        let source_text = "const x = 1;\r\n\r\nconst y = 2;\r\n\r\n\r\nconst z = 3;";
        let source_text = SourceText::new(source_text);
        let comments = vec![];
        let comments = Comments::new(source_text, &comments);
        let file_id = FileId::from_logical_str("formatter/source-crlf.tspp");

        let span_x = Span::new(file_id, 0, 12);
        let span_y = Span::new(file_id, 16, 28);
        let span_z = Span::new(file_id, 34, 46);

        assert_eq!(source_text.text_for(&span_x), "const x = 1;");
        assert_eq!(source_text.text_for(&span_y), "const y = 2;");
        assert_eq!(source_text.text_for(&span_z), "const z = 3;");

        assert_eq!(source_text.get_lines_before(span_y, &comments), 2);
        assert_eq!(source_text.get_lines_before(span_z, &comments), 3);

        assert_eq!(source_text.lines_after(span_x.end), 2);
        assert_eq!(source_text.lines_after(span_y.end), 3);
    }

    /// Keep line counting stable for mixed line endings.
    #[test]
    fn test_source_text_counts_lines_before_for_mixed_line_endings() {
        let source_text = "const x = 1;\n\r\nconst y = 2;\r\n\nconst z = 3;";
        let source_text = SourceText::new(source_text);
        let comments = vec![];
        let comments = Comments::new(source_text, &comments);
        let file_id = FileId::from_logical_str("formatter/source-mixed.tspp");

        let span_x = Span::new(file_id, 0, 12);
        let span_y = Span::new(file_id, 15, 27);
        let span_z = Span::new(file_id, 30, 42);

        assert_eq!(source_text.text_for(&span_x), "const x = 1;");
        assert_eq!(source_text.text_for(&span_y), "const y = 2;");
        assert_eq!(source_text.text_for(&span_z), "const z = 3;");

        assert_eq!(source_text.get_lines_before(span_y, &comments), 2);
        assert_eq!(source_text.get_lines_before(span_z, &comments), 2);

        assert_eq!(source_text.lines_after(span_x.end), 2);
        assert_eq!(source_text.lines_after(span_y.end), 2);
    }
}
