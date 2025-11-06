use std::borrow::Cow;

const LINE_SEPARATOR: char = '\u{2028}';
const PARAGRAPH_SEPARATOR: char = '\u{2029}';
pub const LINE_TERMINATORS: [char; 3] = ['\r', LINE_SEPARATOR, PARAGRAPH_SEPARATOR];

/// Replace the line terminators matching the provided list with "\n".
/// The printer only supports "\n" as line break type.
pub fn normalize_newlines<const N: usize>(text: &str, terminators: [char; N]) -> Cow<'_, str> {
    let mut result = String::new();
    let mut last_end = 0;

    for (start, part) in text.match_indices(terminators) {
        result.push_str(&text[last_end..start]);
        result.push('\n');

        last_end = start + part.len();

        // handle \r\n sequences by skipping the \n
        if part == "\r" && text[last_end..].starts_with('\n') {
            last_end += 1;
        }
    }

    // return original text if no terminators were found
    if result.is_empty() {
        Cow::Borrowed(text)
    } else {
        result.push_str(&text[last_end..text.len()]);
        Cow::Owned(result)
    }
}

/// Lightweight sourcemap marker between source and output tokens.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FileMarker {
    /// Position of the marker in the original source.
    pub source: u32,
    /// Position of the marker in the output code.
    pub dest: u32,
}
