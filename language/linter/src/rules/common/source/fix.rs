use destack_source::Span;

/// Strip one `.member` suffix from a member expression text.
pub fn strip_dot_member_suffix<'a>(text: &'a str, member: &str) -> Option<&'a str> {
    let suffix = format!(".{member}");
    text.strip_suffix(&suffix).map(str::trim_end)
}

/// Expand a statement span to include a trailing semicolon and one line break.
pub fn expand_span_to_statement_terminator(source: &str, span: Span) -> Span {
    let bytes = source.as_bytes();
    let mut end = span.end as usize;

    while end < bytes.len() && matches!(bytes[end], b' ' | b'\t' | b'\r') {
        end += 1;
    }

    if end < bytes.len() && bytes[end] == b';' {
        end += 1;
    }

    if end < bytes.len() && bytes[end] == b'\r' {
        end += 1;
    }

    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    }

    Span::new(span.file, span.start, end as u32)
}

/// Return one statement prefix span ending at semicolon or first newline.
pub fn statement_prefix_span(span: Span, text: &str) -> Option<Span> {
    // find the statement terminator in declaration text
    let statement_length = text
        .find(';')
        .map(|offset| offset + 1)
        .or_else(|| text.find('\n'))
        .unwrap_or(text.len());

    // reject empty spans
    if statement_length == 0 {
        return None;
    }

    // return the leading statement span
    Some(Span::new(
        span.file,
        span.start,
        span.start + statement_length as u32,
    ))
}

/// Build a single quoted string literal with escaped control characters.
pub fn single_quoted_string_literal(text: &str) -> String {
    let mut escaped = String::new();
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '\0' => escaped.push_str("\\0"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\u{000B}' => escaped.push_str("\\v"),
            '\u{000C}' => escaped.push_str("\\f"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(character),
        }
    }

    format!("'{escaped}'")
}

/// Remove the first standalone `async` keyword token and trailing whitespace.
pub fn remove_first_async_keyword(text: &str) -> Option<String> {
    let (async_start, async_end) = first_async_keyword_range(text)?;

    let mut rewritten = String::with_capacity(text.len());
    rewritten.push_str(&text[..async_start]);
    rewritten.push_str(&text[async_end..]);
    if rewritten == text {
        return None;
    }

    Some(rewritten)
}

/// Resolve the first standalone `async` keyword token and trailing whitespace range.
fn first_async_keyword_range(text: &str) -> Option<(usize, usize)> {
    // scan each `async` occurrence and keep the first standalone token
    let mut search_start = 0;
    while let Some(relative_index) = text[search_start..].find("async") {
        let async_start = search_start + relative_index;
        let async_end = async_start + "async".len();

        // require one non identifier boundary before and after `async`
        let before_is_identifier = text[..async_start]
            .chars()
            .next_back()
            .is_some_and(is_identifier_character);
        let after_is_identifier = text[async_end..]
            .chars()
            .next()
            .is_some_and(is_identifier_character);
        if before_is_identifier || after_is_identifier {
            search_start = async_end;
            continue;
        }

        // include trailing whitespace in the removed range
        let mut remove_end = async_end;
        while let Some(character) = text[remove_end..].chars().next() {
            if !character.is_whitespace() {
                break;
            }
            remove_end += character.len_utf8();
        }

        return Some((async_start, remove_end));
    }

    None
}

/// Return true when one character can appear in an identifier token.
fn is_identifier_character(character: char) -> bool {
    character == '_' || character == '$' || character.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Remove the first standalone async keyword and preserve body text.
    #[test]
    fn test_remove_first_async_keyword_removes_token() {
        let rewritten = remove_first_async_keyword("async (value) => value")
            .expect("expected async keyword removal");
        assert_eq!(rewritten, "(value) => value");
    }

    /// Skip identifiers that only contain async as a substring.
    #[test]
    fn test_remove_first_async_keyword_ignores_identifier_substring() {
        let rewritten = remove_first_async_keyword("notasync (value) => value");
        assert_eq!(rewritten, None);
    }
}
