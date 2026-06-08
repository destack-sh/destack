use destack_repository::WarningCommentLocation;
use regex::Regex;

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

/// Return true when one raw source comment is a documentation comment.
pub fn is_doc_comment_source(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("///") || trimmed.starts_with("/**")
}

/// Return true when one source text may contain line or block comments.
pub fn source_text_contains_comment_token(text: &str) -> bool {
    text.contains("//") || text.contains("/*")
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
    fallthrough_comment_matches(text, None)
}

/// Return true when one comment declares intentional switch fallthrough.
pub fn fallthrough_comment_matches(text: &str, pattern: Option<&Regex>) -> bool {
    let normalized = normalize_comment_text(text).to_ascii_lowercase();
    let normalized = normalized.trim();
    if is_directive_comment(normalized) {
        return false;
    }

    if let Some(pattern) = pattern {
        return pattern.is_match(normalized);
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

        let normalized_tag = token.trim_end_matches([':', '.', ',', ';']);

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

/// Return true when one comment contains one warning term with the configured policy.
pub fn comment_contains_warning_term(
    comment: &str,
    term: &str,
    location: WarningCommentLocation,
    decoration: &[String],
) -> bool {
    let term = term.trim();
    if term.is_empty() {
        return false;
    }

    let term_lower = term.to_ascii_lowercase();
    let comment_lower = comment.to_ascii_lowercase();

    if location == WarningCommentLocation::Start {
        let comment_lower = comment_lower.trim_start_matches(|character: char| {
            character.is_ascii_whitespace()
                || decoration.iter().any(|item| {
                    let mut item_characters = item.chars();
                    matches!(
                        (item_characters.next(), item_characters.next()),
                        (Some(item_character), None) if item_character == character
                    )
                })
        });

        return comment_lower.starts_with(term_lower.as_str())
            && warning_term_has_suffix_boundary(comment_lower, term_lower.as_str());
    }

    let starts_with_word = term_lower
        .chars()
        .next()
        .is_some_and(is_warning_term_word_character);
    let ends_with_word = term_lower
        .chars()
        .last()
        .is_some_and(is_warning_term_word_character);

    // scan all occurrences and enforce optional boundaries
    let mut search_start = 0usize;
    while search_start <= comment_lower.len() {
        let Some(relative_match_index) = comment_lower[search_start..].find(term_lower.as_str())
        else {
            return false;
        };
        let match_start = search_start + relative_match_index;
        let match_end = match_start + term_lower.len();

        let previous_character = comment_lower[..match_start].chars().next_back();
        let next_character = comment_lower[match_end..].chars().next();
        let has_prefix_boundary = !starts_with_word
            || previous_character
                .is_none_or(|character| !is_warning_term_word_character(character));
        let has_suffix_boundary = !ends_with_word
            || next_character.is_none_or(|character| !is_warning_term_word_character(character));
        if has_prefix_boundary && has_suffix_boundary {
            return true;
        }

        search_start = match_end;
    }

    false
}

/// Return true when a start-matched warning term has a valid trailing boundary.
fn warning_term_has_suffix_boundary(comment: &str, term: &str) -> bool {
    let ends_with_word = term
        .chars()
        .last()
        .is_some_and(is_warning_term_word_character);
    if !ends_with_word {
        return true;
    }

    comment[term.len()..]
        .chars()
        .next()
        .is_none_or(|character| !is_warning_term_word_character(character))
}

/// Return true when one character counts as a warning term word character.
fn is_warning_term_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
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

    /// Respect custom fallthrough comment patterns.
    #[test]
    fn test_matches_custom_fallthrough_comment_pattern() {
        let pattern =
            Regex::new(r"no break").unwrap_or_else(|error| panic!("expected valid regex: {error}"));
        assert!(fallthrough_comment_matches("// no break", Some(&pattern)));
        assert!(!fallthrough_comment_matches(
            "// fallthrough",
            Some(&pattern)
        ));
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

    /// Match warning terms as whole words.
    #[test]
    fn test_comment_contains_warning_term_matches_whole_word() {
        assert!(comment_contains_warning_term(
            "TODO: finish this path",
            "todo",
            WarningCommentLocation::Anywhere,
            &[],
        ));
        assert!(comment_contains_warning_term(
            "fixme!",
            "fixme",
            WarningCommentLocation::Anywhere,
            &[],
        ));
    }

    /// Skip warning terms that only appear as substrings.
    #[test]
    fn test_comment_contains_warning_term_skips_substring() {
        assert!(!comment_contains_warning_term(
            "TodoMVC integration",
            "todo",
            WarningCommentLocation::Anywhere,
            &[],
        ));
        assert!(!comment_contains_warning_term(
            "prefixfixmesuffix",
            "fixme",
            WarningCommentLocation::Anywhere,
            &[],
        ));
    }

    /// Match warning terms only at the configured start position.
    #[test]
    fn test_comment_contains_warning_term_at_start() {
        assert!(comment_contains_warning_term(
            "   TODO: finish this path",
            "todo",
            WarningCommentLocation::Start,
            &[],
        ));
        assert!(!comment_contains_warning_term(
            "please TODO this later",
            "todo",
            WarningCommentLocation::Start,
            &[],
        ));
    }

    /// Match warning terms after configured decoration characters.
    #[test]
    fn test_comment_contains_warning_term_after_decoration() {
        assert!(comment_contains_warning_term(
            "*** TODO: finish this path",
            "todo",
            WarningCommentLocation::Start,
            &[String::from("*")],
        ));
    }
}
