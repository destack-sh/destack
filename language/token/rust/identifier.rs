use std::borrow::Cow;

use destack_library_unicode::xid::UnicodeXID;

/// Checks if `c` is considered a whitespace according to Unicode `Pattern_White_Space``.
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // usual ASCII suspects
        '\u{0009}'   // \t
			| '\u{000A}' // \n
			| '\u{000B}' // vertical tab
			| '\u{000C}' // form feed
			| '\u{000D}' // \r
			| '\u{0020}' // space
			// NEXT LINE from latin1
			| '\u{0085}'
			// bidi markers
			| '\u{200E}' // LEFT-TO-RIGHT MARK
			| '\u{200F}' // RIGHT-TO-LEFT MARK
			// dedicated whitespace characters from Unicode
			| '\u{2028}' // LINE SEPARATOR
			| '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

/// Checks if `c` is valid as a first character of an identifier.
#[inline]
pub fn is_identifier_start(c: char) -> bool {
    c == '_' || UnicodeXID::is_xid_start(c)
}

/// Checks if `c` is valid as a non-first character of an identifier.
#[inline]
pub fn is_identifier_continue(c: char) -> bool {
    UnicodeXID::is_xid_continue(c)
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
    }

    /// Invalid identifiers should be rejected correctly.
    #[test]
    fn test_is_identifier_invalid() {
        assert!(!is_identifier(""));
        assert!(!is_identifier("123"));
        assert!(!is_identifier("123abc"));
        assert!(!is_identifier("hello-world"));
        assert!(!is_identifier("hello world"));
    }

    /// Already valid identifiers should remain unchanged.
    #[test]
    fn test_to_identifier_valid_input() {
        assert_eq!(to_identifier("foo", '_'), "foo");
        assert_eq!(to_identifier("_bar", '_'), "_bar");
        assert_eq!(to_identifier("baz123", '_'), "baz123");
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
    }

    /// Empty or all-invalid input should return empty string.
    #[test]
    fn test_clean_identifier_empty_result() {
        assert_eq!(clean_identifier(""), "");
        assert_eq!(clean_identifier("123"), "");
        assert_eq!(clean_identifier("@#$"), "");
    }
}
