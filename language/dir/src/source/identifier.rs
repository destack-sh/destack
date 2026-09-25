use std::borrow::Cow;

use tspp_unicode::xid::UnicodeXID;

// Checks for ECMA 262 other identifier start characters.
#[inline]
fn is_other_identifier_start(c: char) -> bool {
    matches!(
        c,
        '\u{0E33}' | '\u{2118}' | '\u{212E}' | '\u{309B}' | '\u{309C}'
    )
}

// Checks for ECMA 262 other identifier continue characters.
#[inline]
fn is_other_identifier_continue(c: char) -> bool {
    matches!(
        c,
        '\u{00B7}' | '\u{0387}' | '\u{1369}'..='\u{1371}' | '\u{19DA}'
    )
}

/// Checks if `c` is considered whitespace per EcmaScript `WhiteSpace` or `LineTerminator`.
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // ascii controls
        '\u{0009}' // horizontal tab
            | '\u{000A}' // line feed
            | '\u{000B}' // vertical tab
            | '\u{000C}' // form feed
            | '\u{000D}' // carriage return
            | '\u{0020}' // space
            // latin-1 additions
            | '\u{0085}' // next line
            | '\u{00A0}' // no-break space
            // ogham + quads/spaces
            | '\u{1680}' // ogham space mark
            | '\u{2000}' // en quad
            | '\u{2001}' // em quad
            | '\u{2002}' // en space
            | '\u{2003}' // em space
            | '\u{2004}' // three-per-em space
            | '\u{2005}' // four-per-em space
            | '\u{2006}' // six-per-em space
            | '\u{2007}' // figure space
            | '\u{2008}' // punctuation space
            | '\u{2009}' // thin space
            | '\u{200A}' // hair space
            | '\u{2028}' // line separator
            | '\u{2029}' // paragraph separator
            | '\u{202F}' // narrow no-break space
            | '\u{205F}' // medium mathematical space
            | '\u{3000}' // ideographic space
            | '\u{FEFF}' // byte order mark
            // bidi markers maintained for compatibility
            | '\u{200E}' // left-to-right mark
            | '\u{200F}' // right-to-left mark
    )
}

/// Checks if `c` is valid as a first character of an identifier.
#[inline]
pub fn is_identifier_start(c: char) -> bool {
    c == '_' || c == '$' || UnicodeXID::is_xid_start(c) || is_other_identifier_start(c)
}

/// Checks if `c` is valid as a non-first character of an identifier.
#[inline]
pub fn is_identifier_continue(c: char) -> bool {
    c == '$'
        || c == '_'
        || UnicodeXID::is_xid_continue(c)
        || is_other_identifier_start(c)
        || is_other_identifier_continue(c)
}

/// Checks if the passed string is lexically an identifier.
#[inline]
pub fn is_identifier(string: &str) -> bool {
    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        is_identifier_start(start) && chars.all(is_identifier_continue)
    } else {
        false
    }
}

/// Checks if the passed string is an identifier under the formatter compatibility rules.
#[inline]
pub fn is_identifier_compat(string: &str) -> bool {
    is_identifier(string) && !string.chars().any(is_compat_rejected_identifier_continue)
}

/// Return whether one identifier continuation character is rejected for compatibility.
#[inline]
fn is_compat_rejected_identifier_continue(c: char) -> bool {
    matches!(c, '\u{30FB}' | '\u{FF65}')
}

/// Converts a string into a valid identifier by replacing invalid characters.
///
/// If the first character is not a valid identifier start, `replacement` is prepended.
/// All invalid characters are replaced with `replacement`.
/// Returns an empty string if the input is empty.
pub fn to_identifier(string: &'_ str, replacement: char) -> Cow<'_, str> {
    // bail if it's already valid
    if is_identifier(string) {
        return Cow::Borrowed(string);
    }

    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        let mut result = String::new();

        // prepend replacement if first character is invalid
        if !is_identifier_start(start) {
            result.push(replacement);
        }

        // handle first character
        if is_identifier_continue(start) {
            result.push(start);
        } else {
            result.push(replacement);
        }

        // handle remaining characters
        for c in chars {
            if is_identifier_continue(c) {
                result.push(c);
            } else {
                result.push(replacement);
            }
        }

        Cow::Owned(result)
    } else {
        Cow::Owned(String::new())
    }
}

/// Converts a string into a valid identifier by removing invalid characters.
///
/// If the first character is not a valid identifier start, it is removed.
/// All invalid characters are removed.
/// Returns an empty string if no valid characters remain.
pub fn clean_identifier(string: &'_ str) -> Cow<'_, str> {
    // bail if it's already valid
    if is_identifier(string) {
        return Cow::Borrowed(string);
    }

    // find the first valid start, then collect valid continues
    let mut iter = string.chars();
    let mut result = String::new();

    // scan until a valid start is found
    for c in iter.by_ref() {
        if is_identifier_start(c) {
            result.push(c);
            break;
        }
    }

    // if no valid start was found, return empty
    if result.is_empty() {
        return Cow::Owned(String::new());
    }

    // collect remaining valid continues
    for c in iter {
        if is_identifier_continue(c) {
            result.push(c);
        }
    }

    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valid identifiers should be recognized correctly.
    #[test]
    fn test_is_identifier_valid() {
        assert!(is_identifier("x"));
        assert!(is_identifier("foo"));
        assert!(is_identifier("_bar"));
        assert!(is_identifier("baz123"));
        assert!(is_identifier("_"));
        assert!(is_identifier("hello_world"));
        assert!(is_identifier("CamelCase"));
        assert!(is_identifier("snake_case"));
        assert!(is_identifier("SCREAMING_SNAKE"));
        assert!(is_identifier("a1b2c3"));
        assert!(is_identifier("$"));
        assert!(is_identifier("$foo"));
        assert!(is_identifier("foo\u{200C}bar"));
        assert!(is_identifier("foo\u{00B7}bar"));
        assert!(is_identifier("\u{309B}"));
        assert!(is_identifier("\u{309C}"));
        assert!(is_identifier("ำ"));
    }

    /// Invalid identifiers should be rejected correctly.
    #[test]
    fn test_is_identifier_invalid() {
        assert!(!is_identifier(""));
        assert!(!is_identifier("123"));
        assert!(!is_identifier("123abc"));
        assert!(!is_identifier("hello-world"));
        assert!(!is_identifier("hello world"));
        assert!(!is_identifier("$ foo"));
    }

    /// Compatibility identifier checks should reject katakana middle dots.
    #[test]
    fn test_is_identifier_compat_rejects_katakana_middle_dot() {
        assert!(is_identifier("x\u{30FB}"));
        assert!(is_identifier("x\u{FF65}"));
        assert!(!is_identifier_compat("x\u{30FB}"));
        assert!(!is_identifier_compat("x\u{FF65}"));
        assert!(is_identifier_compat("foo"));
    }

    /// Already valid identifiers should remain unchanged.
    #[test]
    fn test_to_identifier_valid_input() {
        assert_eq!(to_identifier("foo", '_'), "foo");
        assert_eq!(to_identifier("_bar", '_'), "_bar");
        assert_eq!(to_identifier("baz123", '_'), "baz123");
        assert_eq!(to_identifier("$qux", '_'), "$qux");
        assert_eq!(to_identifier("foo$bar", '_'), "foo$bar");
    }
    /// Invalid start characters should get replacement prepended.
    #[test]
    fn test_to_identifier_invalid_start() {
        assert_eq!(to_identifier("123abc", '_'), "_123abc");
        assert_eq!(to_identifier("9hello", '_'), "_9hello");
        assert_eq!(to_identifier("-world", '_'), "__world");
    }

    /// Invalid characters should be replaced.
    #[test]
    fn test_to_identifier_invalid_characters() {
        assert_eq!(to_identifier("hello-world", '_'), "hello_world");
        assert_eq!(to_identifier("hello world", '_'), "hello_world");
        assert_eq!(to_identifier("hello.world", '_'), "hello_world");
        assert_eq!(to_identifier("hello@world#test", '_'), "hello_world_test");
    }

    /// Empty string should return empty string.
    #[test]
    fn test_to_identifier_empty_string() {
        assert_eq!(to_identifier("", '_'), "");
    }

    /// Test with different replacement characters.
    #[test]
    fn test_to_identifier_different_replacement() {
        assert_eq!(to_identifier("hello-world", 'X'), "helloXworld");
        assert_eq!(to_identifier("123abc", 'Z'), "Z123abc");
    }

    /// Already clean identifiers should remain unchanged.
    #[test]
    fn test_clean_identifier_valid_input() {
        assert_eq!(clean_identifier("foo"), "foo");
        assert_eq!(clean_identifier("_bar"), "_bar");
        assert_eq!(clean_identifier("baz123"), "baz123");
        assert_eq!(clean_identifier("$foo"), "$foo");
        assert_eq!(clean_identifier("foo$bar"), "foo$bar");
    }

    /// Invalid characters should be cleaned out.
    #[test]
    fn test_clean_identifier_invalid_characters() {
        assert_eq!(clean_identifier("hello-world"), "helloworld");
        assert_eq!(clean_identifier("hello world"), "helloworld");
        assert_eq!(clean_identifier("hello.world"), "helloworld");
        assert_eq!(clean_identifier("hello@world#test"), "helloworldtest");
    }

    /// Invalid start characters should be cleaned.
    #[test]
    fn test_clean_identifier_invalid_start() {
        assert_eq!(clean_identifier("123abc"), "abc");
        assert_eq!(clean_identifier("9hello"), "hello");
        assert_eq!(clean_identifier("-world"), "world");
        assert_eq!(clean_identifier("*var T"), "varT");
        assert_eq!(clean_identifier("$ foo"), "$foo");
    }

    /// Empty or all-invalid input should return empty string; otherwise keep the valid suffix.
    #[test]
    fn test_clean_identifier_empty_result() {
        assert_eq!(clean_identifier(""), "");
        assert_eq!(clean_identifier("123"), "");
        assert_eq!(clean_identifier("@#"), "");
        assert_eq!(clean_identifier("@#$"), "$");
    }

    /// EcmaScript whitespace characters should be recognized.
    #[test]
    fn test_is_whitespace_ecmascript() {
        assert!(is_whitespace('\u{0020}'));
        assert!(is_whitespace('\u{00A0}'));
        assert!(is_whitespace('\u{1680}'));
        assert!(is_whitespace('\u{2007}'));
        assert!(is_whitespace('\u{2028}'));
        assert!(is_whitespace('\u{202F}'));
        assert!(is_whitespace('\u{205F}'));
        assert!(is_whitespace('\u{3000}'));
        assert!(is_whitespace('\u{FEFF}'));
        assert!(!is_whitespace('$'));
        assert!(!is_whitespace('a'));
    }
}
