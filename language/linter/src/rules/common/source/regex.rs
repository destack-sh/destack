use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use regex::Regex;

/// Cached compiled regex patterns for `no-fallthrough`.
static NO_FALLTHROUGH_COMMENT_PATTERN_CACHE: LazyLock<Mutex<HashMap<String, Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return a compiled comment pattern for `no-fallthrough`.
pub fn compiled_no_fallthrough_comment_pattern(pattern: Option<&str>) -> Option<Arc<Regex>> {
    let pattern = pattern?;

    let mut cache = NO_FALLTHROUGH_COMMENT_PATTERN_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(compiled_pattern) = cache.get(pattern) {
        return Some(compiled_pattern.clone());
    }

    // validated config invariant
    let compiled_pattern = Arc::new(
        Regex::new(pattern)
            .expect("validated config invariant: no-fallthrough comment pattern compiles"),
    );
    cache.insert(pattern.to_string(), compiled_pattern.clone());
    Some(compiled_pattern)
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
    use super::{regex_prefix_literal, regex_suffix_literal};

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
