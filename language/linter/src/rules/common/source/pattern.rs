/// Return true when one glob pattern matches one text value.
///
/// This supports `*` as a wildcard over zero or more bytes.
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    // fast path: exact match or global wildcard
    if pattern == text || pattern == "*" {
        return true;
    }

    let pattern_bytes = pattern.as_bytes();
    let text_bytes = text.as_bytes();

    let mut pattern_index = 0usize;
    let mut text_index = 0usize;
    let mut last_star_index: Option<usize> = None;
    let mut last_star_match_index = 0usize;

    // scan text with star backtracking
    while text_index < text_bytes.len() {
        // advance both cursors on direct match
        if pattern_index < pattern_bytes.len()
            && pattern_bytes[pattern_index] == text_bytes[text_index]
        {
            pattern_index += 1;
            text_index += 1;
            continue;
        }

        // record wildcard position for later backtracking
        if pattern_index < pattern_bytes.len() && pattern_bytes[pattern_index] == b'*' {
            last_star_index = Some(pattern_index);
            pattern_index += 1;
            last_star_match_index = text_index;
            continue;
        }

        // backtrack to last wildcard and consume one more text byte
        if let Some(star_index) = last_star_index {
            pattern_index = star_index + 1;
            last_star_match_index += 1;
            text_index = last_star_match_index;
            continue;
        }

        return false;
    }

    // trailing wildcards match an empty suffix
    while pattern_index < pattern_bytes.len() && pattern_bytes[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern_bytes.len()
}

/// Return one plain prefix string from a regex pattern like `^text`.
pub fn regex_prefix_literal(pattern: &str, flags: &str) -> Option<String> {
    // reject regex flags that alter prefix matching behavior
    if flags.contains('i') || flags.contains('m') {
        return None;
    }

    // require one anchored prefix pattern
    let prefix = pattern.strip_prefix('^')?;
    if !regex_literal_is_simple(prefix) {
        return None;
    }

    Some(prefix.to_string())
}

/// Return one plain suffix string from a regex pattern like `text$`.
pub fn regex_suffix_literal(pattern: &str, flags: &str) -> Option<String> {
    // reject regex flags that alter suffix matching behavior
    if flags.contains('i') || flags.contains('m') {
        return None;
    }

    // require one anchored suffix pattern
    let suffix = pattern.strip_suffix('$')?;
    if !regex_literal_is_simple(suffix) {
        return None;
    }

    Some(suffix.to_string())
}

/// Return true when one regex fragment has no metacharacters.
fn regex_literal_is_simple(fragment: &str) -> bool {
    !fragment.chars().any(|character| {
        matches!(
            character,
            '^' | '$' | '+' | '[' | '{' | '(' | '\\' | '.' | '?' | '*' | '|'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{glob_matches, regex_prefix_literal, regex_suffix_literal};

    /// Match exact text with no wildcard.
    #[test]
    fn test_matches_exact_pattern() {
        assert!(glob_matches("core/utils", "core/utils"));
    }

    /// Match a global wildcard.
    #[test]
    fn test_matches_global_wildcard() {
        assert!(glob_matches("*", "anything/goes"));
    }

    /// Match prefix wildcard patterns.
    #[test]
    fn test_matches_prefix_pattern() {
        assert!(glob_matches("internal/*", "internal/http/client"));
    }

    /// Match suffix wildcard patterns.
    #[test]
    fn test_matches_suffix_pattern() {
        assert!(glob_matches("*.gen.ds", "user.gen.ds"));
    }

    /// Match infix wildcard patterns.
    #[test]
    fn test_matches_infix_pattern() {
        assert!(glob_matches("lib/*/unsafe", "lib/sql/unsafe"));
    }

    /// Reject text that does not match the pattern.
    #[test]
    fn test_rejects_non_matching_pattern() {
        assert!(!glob_matches("internal/*", "external/client"));
    }

    /// Extract one simple regex prefix literal.
    #[test]
    fn test_extracts_regex_prefix_literal() {
        assert_eq!(regex_prefix_literal("^foo", ""), Some("foo".to_string()));
    }

    /// Extract one simple regex suffix literal.
    #[test]
    fn test_extracts_regex_suffix_literal() {
        assert_eq!(regex_suffix_literal("foo$", ""), Some("foo".to_string()));
    }

    /// Reject complex regex prefix patterns.
    #[test]
    fn test_rejects_complex_regex_prefix_literal() {
        assert_eq!(regex_prefix_literal("^fo+", ""), None);
    }

    /// Reject case insensitive regex prefix patterns.
    #[test]
    fn test_rejects_case_insensitive_regex_prefix_literal() {
        assert_eq!(regex_prefix_literal("^foo", "i"), None);
    }
}
