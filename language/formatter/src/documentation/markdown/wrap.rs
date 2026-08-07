use std::borrow::Cow;

use super::super::line::LineBuffer;
use super::MarkdownFormatter;

/// Preallocated indentation from zero through twelve spaces.
const INDENTATION: [&str; 13] = [
    "",
    " ",
    "  ",
    "   ",
    "    ",
    "     ",
    "      ",
    "       ",
    "        ",
    "         ",
    "          ",
    "           ",
    "            ",
];

impl MarkdownFormatter<'_> {
    /// Return the character width of one string.
    #[inline]
    pub(super) fn text_width(text: &str) -> usize {
        if text.is_ascii() {
            text.len()
        } else {
            text.chars().count()
        }
    }

    /// Return indentation of one exact width.
    pub(super) fn indentation_text(width: usize) -> Cow<'static, str> {
        if let Some(indentation) = INDENTATION.get(width).copied() {
            Cow::Borrowed(indentation)
        } else {
            Cow::Owned(" ".repeat(width))
        }
    }

    /// Wrap one Markdown paragraph.
    pub(super) fn wrap_paragraph(
        &self,
        text: &str,
        width: usize,
        first_line_offset: usize,
        continuation_indent: usize,
        lines: &mut LineBuffer,
    ) {
        let words = Self::words(text);
        if words.is_empty() {
            return;
        }

        let first_width = width.saturating_sub(first_line_offset);
        let continuation_width = width.saturating_sub(continuation_indent);
        let indentation = Self::indentation_text(continuation_indent);
        let mut line = String::with_capacity(width);
        let mut line_width = 0;
        let mut is_first = true;

        // append words until the current line is full
        for word in words {
            let word_width = Self::text_width(word);
            let available = if is_first {
                first_width
            } else {
                continuation_width
            };
            let separator_width = usize::from(!line.is_empty());
            if !line.is_empty() && line_width + separator_width + word_width > available {
                Self::push_wrapped_line(&line, is_first, &indentation, lines);
                line.clear();
                line_width = 0;
                is_first = false;
            }

            if !line.is_empty() {
                line.push(' ');
                line_width += 1;
            }
            line.push_str(word);
            line_width += word_width;
        }

        // append the remaining line
        if !line.is_empty() {
            Self::push_wrapped_line(&line, is_first, &indentation, lines);
        }
    }

    /// Split prose into words without splitting inline links.
    fn words(text: &str) -> Vec<&str> {
        let bytes = text.as_bytes();
        let mut words = Vec::new();
        let mut index = 0;

        while index < bytes.len() {
            // skip whitespace between words
            while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            if index == bytes.len() {
                break;
            }

            // retain documentation tags and Markdown links as one word
            let start = index;
            index = if bytes[index..].starts_with(b"{@") {
                let end = Self::delimited_end(bytes, index, b'{', b'}');

                Self::word_end(bytes, end)
            } else if bytes[index..].starts_with(b"![") {
                Self::link_end(bytes, index + 1)
            } else if bytes[index] == b'[' {
                Self::link_end(bytes, index)
            } else {
                Self::word_end(bytes, index)
            };
            words.push(&text[start..index]);
        }

        words
    }

    /// Return the end of one ordinary word.
    fn word_end(bytes: &[u8], mut index: usize) -> usize {
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        index
    }

    /// Return the end of one Markdown link or reference.
    fn link_end(bytes: &[u8], start: usize) -> usize {
        let label_end = Self::delimited_end(bytes, start, b'[', b']');
        if label_end < bytes.len() && bytes[label_end] == b'(' {
            let destination_end = Self::delimited_end(bytes, label_end, b'(', b')');

            Self::word_end(bytes, destination_end)
        } else if label_end < bytes.len() && bytes[label_end] == b'[' {
            let reference_end = Self::delimited_end(bytes, label_end, b'[', b']');

            Self::word_end(bytes, reference_end)
        } else {
            Self::word_end(bytes, label_end)
        }
    }

    /// Return the end of one balanced byte-delimited sequence.
    fn delimited_end(bytes: &[u8], mut index: usize, open: u8, close: u8) -> usize {
        let mut depth = 0;
        while index < bytes.len() {
            if bytes[index] == open {
                depth += 1;
            } else if bytes[index] == close {
                depth -= 1;
                if depth == 0 {
                    index += 1;

                    return index;
                }
            }
            index += 1;
        }

        index
    }

    /// Append one wrapped line with its continuation indentation.
    fn push_wrapped_line(line: &str, is_first: bool, indentation: &str, lines: &mut LineBuffer) {
        if is_first {
            lines.push(line);
        } else {
            let output = lines.begin_line();
            output.push_str(indentation);
            output.push_str(line);
        }
    }
}
