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

#[cfg(test)]
mod tests {
    use super::glob_matches;

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
}
