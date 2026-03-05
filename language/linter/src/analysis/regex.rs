use std::sync::Arc;

use regex_syntax::ast::ErrorKind as AstErrorKind;
use regex_syntax::hir::{ErrorKind as HirErrorKind, Hir};
use regex_syntax::{Error as RegexError, Parser};

/// The kind of error produced while parsing a regex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintRegexErrorKind {
    /// The regex failed while parsing into an AST.
    Parse(AstErrorKind),
    /// The regex failed while translating the AST into HIR.
    Translate(HirErrorKind),
    /// The regex failed with an unclassified error.
    Other,
}

/// Describes a regex parse error for lint rules.
#[derive(Debug, Clone)]
pub struct LintRegexError {
    /// The error kind reported by the regex parser.
    pub kind: LintRegexErrorKind,
    /// A formatted error message for diagnostics.
    pub message: String,
}

/// Describe the parsed regex data for lint rules.
#[derive(Debug, Clone)]
pub struct LintRegexParse {
    /// The parsed regex HIR when available.
    pub hir: Option<Arc<Hir>>,
    /// The parse error when the pattern is invalid.
    pub error: Option<LintRegexError>,
}

impl LintRegexParse {
    /// Parse a pattern and return cached regex data.
    pub fn parse(pattern: &str) -> Self {
        let parse_result = Parser::new().parse(pattern);
        let (hir, error) = match parse_result {
            Ok(hir) => (Some(Arc::new(hir)), None),
            Err(err) => {
                let message = err.to_string();
                let kind = match err {
                    RegexError::Parse(parse_error) => {
                        LintRegexErrorKind::Parse(parse_error.kind().clone())
                    }
                    RegexError::Translate(translate_error) => {
                        LintRegexErrorKind::Translate(translate_error.kind().clone())
                    }
                    _ => LintRegexErrorKind::Other,
                };
                (None, Some(LintRegexError { kind, message }))
            }
        };

        Self { hir, error }
    }
}

/// Find a control character in a raw pattern string.
pub(crate) fn find_control_character(s: &str) -> Option<char> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            match next {
                'c' => {
                    i += 3;
                    continue;
                }
                'x' => {
                    i += 4;
                    continue;
                }
                'u' => {
                    i += 6;
                    continue;
                }
                '0' => {
                    i += 2;
                    continue;
                }
                'n' | 'r' | 't' | 'f' | 'v' => {
                    i += 2;
                    continue;
                }
                _ => {
                    i += 2;
                    continue;
                }
            }
        }

        if c.is_ascii_control() && c != '\t' && c != '\n' && c != '\r' {
            return Some(c);
        }

        i += 1;
    }

    None
}

/// Find control characters in a regex pattern with optional flag semantics.
pub(crate) fn find_control_characters(pattern: &str, flags: Option<&str>) -> Vec<String> {
    let unicode_mode = flags.is_some_and(|flags| flags.contains('u') || flags.contains('v'));
    let bytes = pattern.as_bytes();
    let mut control_characters = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += collect_control_escape(&mut control_characters, bytes, index, unicode_mode);
            continue;
        }

        if let Some(character) = pattern[index..].chars().next() {
            if character.is_ascii_control() {
                push_control_character(&mut control_characters, character as u32);
            }
            index += character.len_utf8();
            continue;
        }

        break;
    }

    control_characters
}

/// Parse one regex escape sequence and collect control characters.
fn collect_control_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
    unicode_mode: bool,
) -> usize {
    if index + 1 >= bytes.len() {
        return 1;
    }

    match bytes[index + 1] {
        b'x' => collect_hex_escape(control_characters, bytes, index),
        b'u' => collect_unicode_escape(control_characters, bytes, index, unicode_mode),
        _ => 2,
    }
}

/// Parse `\xNN` escape and collect a control character when present.
fn collect_hex_escape(control_characters: &mut Vec<String>, bytes: &[u8], index: usize) -> usize {
    if index + 3 >= bytes.len() {
        return 2;
    }

    let high = hex_nibble(bytes[index + 2]);
    let low = hex_nibble(bytes[index + 3]);
    let (Some(high), Some(low)) = (high, low) else {
        return 2;
    };

    let value = (high << 4) | low;
    push_control_character(control_characters, value as u32);
    4
}

/// Parse `\uNNNN` or `\u{...}` escape and collect a control character when present.
fn collect_unicode_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
    unicode_mode: bool,
) -> usize {
    if index + 2 >= bytes.len() {
        return 2;
    }

    if bytes[index + 2] == b'{' {
        if !unicode_mode {
            return 2;
        }
        return collect_unicode_braced_escape(control_characters, bytes, index);
    }

    if index + 5 >= bytes.len() {
        return 2;
    }

    let mut value = 0u32;
    for offset in 0..4 {
        let Some(nibble) = hex_nibble(bytes[index + 2 + offset]) else {
            return 2;
        };
        value = (value << 4) | nibble as u32;
    }

    push_control_character(control_characters, value);
    6
}

/// Parse `\u{...}` escape and collect a control character when present.
fn collect_unicode_braced_escape(
    control_characters: &mut Vec<String>,
    bytes: &[u8],
    index: usize,
) -> usize {
    let mut cursor = index + 3;
    let mut value = 0u32;
    let mut has_digit = false;

    while cursor < bytes.len() {
        if bytes[cursor] == b'}' {
            if has_digit {
                push_control_character(control_characters, value);
                return cursor - index + 1;
            }
            return 2;
        }

        let Some(nibble) = hex_nibble(bytes[cursor]) else {
            return 2;
        };
        has_digit = true;
        value = (value << 4) | nibble as u32;
        cursor += 1;
    }

    2
}

/// Convert one ASCII hex byte to its numeric nibble value.
fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Add one control character to the collected diagnostics list.
fn push_control_character(control_characters: &mut Vec<String>, value: u32) {
    if value <= 0x1f {
        control_characters.push(format!("\\x{value:02x}"));
    }
}

/// Find a misleading character class in a pattern string.
pub(crate) fn find_misleading_character_class(regex: &str) -> Option<&'static str> {
    let mut in_class = false;
    let mut chars = regex.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '[' if !in_class => {
                in_class = true;
            }
            ']' if in_class => {
                in_class = false;
            }
            _ if in_class => {
                if is_combining_mark(c) {
                    return Some("combining character");
                }

                if c >= '\u{1F1E6}' && c <= '\u{1F1FF}' {
                    return Some("regional indicator symbol");
                }

                if is_zero_width(c) {
                    return Some("zero-width character");
                }
            }
            _ => {}
        }
    }

    None
}

/// Return whether the character is a combining mark.
fn is_combining_mark(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' |
        '\u{1AB0}'..='\u{1AFF}' |
        '\u{1DC0}'..='\u{1DFF}' |
        '\u{20D0}'..='\u{20FF}' |
        '\u{FE20}'..='\u{FE2F}'
    )
}

/// Return whether the character is zero width.
fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}')
}

/// Find a backreference that cannot resolve to a prior group.
pub(crate) fn find_useless_backreference(
    regex: &str,
    error_kind: Option<&LintRegexErrorKind>,
) -> Option<String> {
    let Some(LintRegexErrorKind::Parse(kind)) = error_kind else {
        return None;
    };
    if matches!(kind, AstErrorKind::UnsupportedBackreference) {
        return analyze_backreferences(regex);
    }
    None
}

/// Analyze backreference usage for invalid references.
fn analyze_backreferences(regex: &str) -> Option<String> {
    let mut group_count = 0;
    let mut chars = regex.chars().peekable();
    let mut in_char_class = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() && next != '0' && !in_char_class {
                        chars.next();
                        let mut num_str = String::from(next);

                        while let Some(&d) = chars.peek() {
                            if d.is_ascii_digit() {
                                num_str.push(d);
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        if let Ok(backref_num) = num_str.parse::<usize>()
                            && backref_num > group_count
                        {
                            return Some(format!(
                                "backreference \\{backref_num} references non-existent group \
                                     (only {group_count} groups defined so far)"
                            ));
                        }
                    } else {
                        chars.next();
                    }
                }
            }
            '[' if !in_char_class => {
                in_char_class = true;
            }
            ']' if in_char_class => {
                in_char_class = false;
            }
            '(' if !in_char_class => {
                if chars.peek() != Some(&'?') {
                    group_count += 1;
                } else {
                    let mut temp_chars = chars.clone();
                    temp_chars.next();
                    if temp_chars.peek() == Some(&'<') {
                        temp_chars.next();
                        if let Some(&c) = temp_chars.peek()
                            && c != '='
                            && c != '!'
                        {
                            group_count += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    None
}
