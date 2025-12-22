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
