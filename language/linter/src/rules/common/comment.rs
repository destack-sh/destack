/// Parsed keyword comment metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordCommentInfo {
    /// The canonical keyword for this comment.
    pub keyword: String,
    /// Whether the keyword token is uppercase in source text.
    pub keyword_is_uppercase: bool,
    /// Whether this comment contains at least one known tag.
    pub has_known_tag: bool,
    /// Whether this comment contains an unknown tag.
    pub has_unknown_tag: bool,
}

/// Return the first alphabetic character in one string.
pub fn first_alphabetic_character(text: &str) -> Option<char> {
    text.chars().find(|character| character.is_alphabetic())
}

/// Return true when one comment is a separator line.
pub fn is_separator_comment(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return false;
    }

    let mut has_separator_character = false;
    for character in trimmed.chars() {
        if matches!(character, '=' | '-' | '_' | ' ') {
            has_separator_character |= character != ' ';
            continue;
        }

        return false;
    }

    has_separator_character
}

/// Return true when one comment line is a separator heading between separator lines.
pub fn is_separator_heading_line(
    previous_text: Option<&str>,
    current_text: &str,
    next_text: Option<&str>,
    min_lines: usize,
) -> bool {
    if min_lines > 3 {
        return false;
    }

    let Some(previous_text) = previous_text else {
        return false;
    };
    let Some(next_text) = next_text else {
        return false;
    };

    let previous_text = previous_text.trim();
    let current_text = current_text.trim();
    let next_text = next_text.trim();

    if current_text.is_empty() || is_separator_comment(current_text) {
        return false;
    }

    is_separator_comment(previous_text) && is_separator_comment(next_text)
}

/// Return true when one comment starts with a known directive marker.
pub fn is_directive_comment(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with(['@', '!'])
        || trimmed.starts_with("#region")
        || trimmed.starts_with("#endregion")
        || trimmed.starts_with("eslint-")
}

/// Return true when one comment declares intentional switch fallthrough.
pub fn is_fallthrough_comment(text: &str) -> bool {
    let normalized = normalize_comment_text(text).to_ascii_lowercase();
    let normalized = normalized.trim();
    if is_directive_comment(normalized) {
        return false;
    }

    normalized.contains("fallthrough")
        || normalized.contains("fall through")
        || normalized.contains("falls through")
}

/// Normalize one comment text to raw payload words.
fn normalize_comment_text(text: &str) -> &str {
    let text = text.trim();
    let text = text.strip_prefix("//").unwrap_or(text);
    let text = text.strip_prefix("/*").unwrap_or(text);
    let text = text.strip_suffix("*/").unwrap_or(text);
    text.trim()
}

/// Parse one keyword comment prefix and its tags using configured options.
pub fn parse_keyword_comment_with_options(
    text: &str,
    keywords: &[String],
    tags: &[String],
) -> Option<KeywordCommentInfo> {
    let trimmed = text.trim_start();
    let first_token = trimmed.split_whitespace().next()?;
    let first_token = first_token.trim_end_matches(':');

    let keyword = keywords
        .iter()
        .find(|keyword| first_token.eq_ignore_ascii_case(keyword.as_str()))?;
    let keyword_is_uppercase = first_token == keyword;

    let mut has_known_tag = false;
    let mut has_unknown_tag = false;

    for token in trimmed.split_whitespace().skip(1) {
        if !token.starts_with('#') {
            continue;
        }

        let normalized_tag =
            token.trim_end_matches(|character: char| matches!(character, ':' | '.' | ',' | ';'));

        if is_known_tag(normalized_tag, tags) {
            has_known_tag = true;
            continue;
        }

        has_unknown_tag = true;
    }

    Some(KeywordCommentInfo {
        keyword: keyword.clone(),
        keyword_is_uppercase,
        has_known_tag,
        has_unknown_tag,
    })
}

/// Return true when one normalized tag matches configured tags.
fn is_known_tag(normalized_tag: &str, tags: &[String]) -> bool {
    let normalized_tag_without_hash = normalized_tag.trim_start_matches('#');
    tags.iter().any(|configured_tag| {
        let configured_tag_without_hash = configured_tag.trim_start_matches('#');
        normalized_tag_without_hash.eq(configured_tag_without_hash)
    })
}

/// Return true when one line contains sentence punctuation followed by new prose.
pub fn has_multiple_sentence_starts(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return false;
    }

    let mut index = 0usize;
    while index + 2 < bytes.len() {
        let byte = bytes[index];
        if !matches!(byte, b'.' | b'!' | b'?') {
            index += 1;
            continue;
        }

        if !bytes[index + 1].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        let mut lookahead_index = index + 2;
        while lookahead_index < bytes.len() && bytes[lookahead_index].is_ascii_whitespace() {
            lookahead_index += 1;
        }

        if lookahead_index >= bytes.len() {
            return false;
        }

        if bytes[lookahead_index].is_ascii_alphabetic() {
            return true;
        }

        index += 1;
    }

    false
}

/// Return true when one comment uses a spaced hyphen separator.
pub fn has_hyphen_separator(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.contains("://") || trimmed.contains('`') {
        return false;
    }

    trimmed.contains(" - ")
}

/// Return true when one documentation line should be excluded from prose checks.
pub fn is_non_prose_doc_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }

    if trimmed.starts_with("```")
        || trimmed.starts_with('@')
        || trimmed.starts_with('`')
        || trimmed.starts_with('|')
    {
        return true;
    }

    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }

    let mut characters = trimmed.chars();
    let Some(first_character) = characters.next() else {
        return true;
    };

    first_character.is_ascii_digit() && matches!(characters.next(), Some('.'))
}

/// Return true when one documentation line ends with sentence punctuation.
pub fn has_doc_terminal_punctuation(line: &str) -> bool {
    let trimmed = line.trim_end();
    if trimmed.ends_with(['.', '!', '?', ':']) {
        return true;
    }

    if !trimmed.ends_with(')') {
        return false;
    }

    let without_parenthesis = trimmed.trim_end_matches(')').trim_end();
    without_parenthesis.ends_with(['.', '!', '?', ':'])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Return test keyword prefixes.
    fn test_keywords() -> Vec<String> {
        vec!["ABC".to_string(), "XYZ".to_string()]
    }

    /// Return test keyword tags.
    fn test_keyword_tags() -> Vec<String> {
        vec![
            "#Performance".to_string(),
            "#Robustness".to_string(),
            "#Broken".to_string(),
            "#Cleanup".to_string(),
            "#Incomplete".to_string(),
            "#Suspicious".to_string(),
            "#Security".to_string(),
            "#Architecture".to_string(),
        ]
    }

    /// Parse uppercase keyword comments with known tags.
    #[test]
    fn test_parse_keyword_comment_with_known_tag() {
        let keywords = test_keywords();
        let tags = test_keyword_tags();
        let info = parse_keyword_comment_with_options(
            "ABC #Suspicious: this allocation might be expensive",
            &keywords,
            &tags,
        )
        .unwrap();
        assert_eq!(info.keyword, "ABC");
        assert!(info.keyword_is_uppercase);
        assert!(info.has_known_tag);
        assert!(!info.has_unknown_tag);
    }

    /// Parse lowercase keyword comments and preserve uppercase metadata.
    #[test]
    fn test_parse_keyword_comment_lowercase_keyword() {
        let keywords = test_keywords();
        let tags = test_keyword_tags();
        let info = parse_keyword_comment_with_options(
            "xyz #Cleanup: refactor this path",
            &keywords,
            &tags,
        )
        .unwrap();
        assert_eq!(info.keyword, "XYZ");
        assert!(!info.keyword_is_uppercase);
        assert!(info.has_known_tag);
        assert!(!info.has_unknown_tag);
    }

    /// Parse keyword comments with unknown tags.
    #[test]
    fn test_parse_keyword_comment_with_unknown_tag() {
        let keywords = test_keywords();
        let tags = test_keyword_tags();
        let info =
            parse_keyword_comment_with_options("XYZ #Whatever: unknown tag", &keywords, &tags)
                .unwrap();
        assert_eq!(info.keyword, "XYZ");
        assert!(info.keyword_is_uppercase);
        assert!(!info.has_known_tag);
        assert!(info.has_unknown_tag);
    }

    /// Detect multiple sentence starts on one line.
    #[test]
    fn test_detects_multiple_sentence_starts() {
        assert!(has_multiple_sentence_starts(
            "This is one sentence. this is another sentence"
        ));
    }

    /// Skip single sentence lines.
    #[test]
    fn test_skips_single_sentence_line() {
        assert!(!has_multiple_sentence_starts("This is a single sentence."));
    }

    /// Detect separator comments.
    #[test]
    fn test_detects_separator_comment() {
        assert!(is_separator_comment(
            "================================================================================"
        ));
    }

    /// Skip prose comments for separator detection.
    #[test]
    fn test_skips_separator_detection_for_prose() {
        assert!(!is_separator_comment("Binary operator precedence"));
    }

    /// Detect intentional fallthrough comments.
    #[test]
    fn test_detects_fallthrough_comment() {
        assert!(is_fallthrough_comment("// falls through"));
        assert!(is_fallthrough_comment("/* fallthrough */"));
        assert!(is_fallthrough_comment("// fall through"));
    }

    /// Skip non fallthrough comments.
    #[test]
    fn test_skips_non_fallthrough_comment() {
        assert!(!is_fallthrough_comment("// continue below"));
    }

    /// Skip directive comments that include fallthrough text.
    #[test]
    fn test_skips_fallthrough_directive_comment() {
        assert!(!is_fallthrough_comment(
            "// eslint-disable-next-line no-fallthrough"
        ));
    }
}
