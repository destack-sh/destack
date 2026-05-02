use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Error produced while applying text changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextChangeError {
    /// A range start comes after its end.
    RangeOrder,
    /// A range does not align to UTF-8 character boundaries.
    CharacterBoundary,
    /// A line is outside the current text.
    LineOutOfRange {
        /// The requested zero-based line.
        line: u32,
    },
    /// A UTF-16 column splits one scalar value.
    ScalarSplit {
        /// The requested zero-based UTF-16 column.
        character: u32,
    },
    /// A UTF-16 column is outside the requested line.
    CharacterOutOfRange {
        /// The requested zero-based UTF-16 column.
        character: u32,
    },
}

impl Display for TextChangeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RangeOrder => {
                write!(formatter, "text change range start is after range end")
            }
            Self::CharacterBoundary => {
                write!(
                    formatter,
                    "text change range does not align to character boundaries"
                )
            }
            Self::LineOutOfRange { line } => {
                write!(formatter, "text change line is out of range: {line}")
            }
            Self::ScalarSplit { character } => {
                write!(
                    formatter,
                    "text change column splits a UTF-16 scalar: {character}"
                )
            }
            Self::CharacterOutOfRange { character } => {
                write!(formatter, "text change column is out of range: {character}")
            }
        }
    }
}

impl Error for TextChangeError {}

/// One textual edit in an open text buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChange {
    /// Optional range to replace, absent for full replacement.
    pub range: Option<TextRange>,
    /// Replacement text.
    pub text: String,
}

/// One text range expressed in UTF-16 positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    /// Start position.
    pub start: TextPosition,
    /// End position.
    pub end: TextPosition,
}

/// One zero-based UTF-16 text position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based UTF-16 column.
    pub character: u32,
}

/// Return text after applying ordered text changes.
pub fn apply_text_changes(
    mut text: String,
    changes: &[TextChange],
) -> Result<String, TextChangeError> {
    // apply changes in protocol order to the current buffer
    for change in changes {
        apply_text_change(&mut text, change)?;
    }

    Ok(text)
}

/// Apply one text change to one text buffer.
pub fn apply_text_change(text: &mut String, change: &TextChange) -> Result<(), TextChangeError> {
    // normalize incoming replacement payloads
    let change_text = normalize_line_endings(change.text.clone());
    let Some(range) = change.range else {
        *text = change_text;

        return Ok(());
    };

    // translate utf16 positions into byte offsets for the current text
    let line_offsets = line_start_offsets(text);
    let start = position_to_byte(text, &line_offsets, range.start)?;
    let end = position_to_byte(text, &line_offsets, range.end)?;
    if start > end {
        return Err(TextChangeError::RangeOrder);
    }

    // reject invalid patch boundaries loudly
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return Err(TextChangeError::CharacterBoundary);
    }

    text.replace_range(start..end, &change_text);

    Ok(())
}

/// Normalize line endings to LF.
fn normalize_line_endings(content: String) -> String {
    // fast path when no normalization is needed
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}

/// Build line start offsets for one text buffer.
fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];

    // record the byte after each line break
    for (index, character) in text.char_indices() {
        if character == '\n' {
            offsets.push(index + 1);
        }
    }

    offsets
}

/// Convert one UTF-16 text position into a byte offset.
fn position_to_byte(
    text: &str,
    line_start_offsets: &[usize],
    position: TextPosition,
) -> Result<usize, TextChangeError> {
    // resolve the requested line
    let line_index = position.line as usize;
    let Some(line_start) = line_start_offsets.get(line_index).copied() else {
        return Err(TextChangeError::LineOutOfRange {
            line: position.line,
        });
    };

    // compute the byte range for the line without its line break
    let mut line_end = match line_start_offsets.get(line_index + 1).copied() {
        Some(next_start) => next_start.saturating_sub(1),
        None => text.len(),
    };
    if line_end > line_start && text.as_bytes()[line_end - 1] == b'\r' {
        line_end -= 1;
    }
    let slice = &text[line_start..line_end];

    // walk characters by utf16 units
    let mut utf16_units = 0u32;
    let mut byte_offset = 0usize;
    for character in slice.chars() {
        if utf16_units >= position.character {
            break;
        }

        let character_units = character.len_utf16() as u32;
        if utf16_units + character_units > position.character {
            return Err(TextChangeError::ScalarSplit {
                character: position.character,
            });
        }

        utf16_units += character_units;
        byte_offset += character.len_utf8();
    }

    // reject positions past the end of the line
    if utf16_units < position.character {
        return Err(TextChangeError::CharacterOutOfRange {
            character: position.character,
        });
    }

    Ok(line_start + byte_offset)
}

#[cfg(test)]
mod tests {
    use super::{TextChange, TextPosition, TextRange, apply_text_changes};

    #[test]
    fn test_apply_text_changes_handles_utf16_positions() {
        let changes = vec![TextChange {
            range: Some(TextRange {
                start: TextPosition {
                    line: 0,
                    character: 3,
                },
                end: TextPosition {
                    line: 0,
                    character: 4,
                },
            }),
            text: "d".to_string(),
        }];

        let text = apply_text_changes("a😀c".to_string(), &changes).unwrap();

        assert_eq!(text, "a😀d");
    }

    #[test]
    fn test_apply_text_changes_handles_crlf_lines() {
        let changes = vec![TextChange {
            range: Some(TextRange {
                start: TextPosition {
                    line: 1,
                    character: 3,
                },
                end: TextPosition {
                    line: 1,
                    character: 3,
                },
            }),
            text: "!".to_string(),
        }];

        let text = apply_text_changes("one\r\ntwo\r\n".to_string(), &changes).unwrap();

        assert_eq!(text, "one\r\ntwo!\r\n");
    }
}
