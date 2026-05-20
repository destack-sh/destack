use std::borrow::Cow;

/// Normalize Jsdoc tag aliases to their canonical spelling.
pub(super) fn normalize_tag_kind(kind: &str) -> &str {
    match kind {
        "return" => "returns",
        "arg" | "argument" | "params" => "param",
        "yield" => "yields",
        "prop" => "property",
        "constructor" => "class",
        "const" => "constant",
        "desc" => "description",
        "host" => "external",
        "fileoverview" | "overview" => "file",
        "emits" => "fires",
        "func" | "method" => "function",
        "var" => "member",
        "virtual" => "abstract",
        "exception" => "throws",
        "examples" => "example",
        "hidden" => "ignore",
        _ => kind,
    }
}

/// Normalize markdown emphasis markers outside inline code spans.
pub(super) fn normalize_markdown_emphasis(text: &str) -> Cow<'_, str> {
    if !text.contains("__") && !text.contains('*') {
        return Cow::Borrowed(text);
    }

    if !emphasis_needs_change(text.as_bytes()) {
        return Cow::Borrowed(text);
    }

    let normalized = replace_double_underscore(text);
    let normalized = replace_single_asterisk(&normalized);

    Cow::Owned(normalized)
}

/// Return whether markdown emphasis normalization would change the bytes.
fn emphasis_needs_change(bytes: &[u8]) -> bool {
    let mut index = 0;
    let mut is_in_code = false;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            is_in_code = !is_in_code;
            index += 1;
            continue;
        }

        if is_in_code {
            index += 1;
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'_' && bytes[index + 1] == b'_' {
            return true;
        }

        index += 1;
    }

    index = 0;
    is_in_code = false;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            is_in_code = !is_in_code;
            index += 1;
            continue;
        }

        if is_in_code {
            index += 1;
            continue;
        }

        if index + 1 < bytes.len()
            && (bytes[index] == b'*' || bytes[index] == b'_')
            && bytes[index + 1] == bytes[index]
        {
            index += 2;
            continue;
        }

        if bytes[index] == b'*' && has_closing_single_asterisk(bytes, index) {
            return true;
        }

        index += 1;
    }

    false
}

/// Replace double underscores with markdown bold asterisks outside inline code.
fn replace_double_underscore(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let mut is_in_code = false;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            is_in_code = !is_in_code;
            output.push('`');
            index += 1;
            continue;
        }

        if is_in_code {
            push_next_char(text, &mut output, &mut index);
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'_' && bytes[index + 1] == b'_' {
            output.push_str("**");
            index += 2;
            continue;
        }

        push_next_char(text, &mut output, &mut index);
    }

    output
}

/// Replace single asterisk emphasis with underscores outside inline code.
fn replace_single_asterisk(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let mut is_in_code = false;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            is_in_code = !is_in_code;
            output.push('`');
            index += 1;
            continue;
        }

        if is_in_code {
            push_next_char(text, &mut output, &mut index);
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'*' && bytes[index + 1] == b'*' {
            output.push_str("**");
            index += 2;
            continue;
        }

        if bytes[index] == b'*'
            && let Some(close_index) = closing_single_asterisk(bytes, index)
        {
            output.push('_');
            output.push_str(&text[index + 1..close_index]);
            output.push('_');
            index = close_index + 1;
            continue;
        }

        push_next_char(text, &mut output, &mut index);
    }

    output
}

/// Push the next UTF-8 character from one byte cursor.
fn push_next_char(text: &str, output: &mut String, index: &mut usize) {
    let Some(character) = text[*index..].chars().next() else {
        return;
    };

    output.push(character);
    *index += character.len_utf8();
}

/// Return whether one single asterisk starts a balanced emphasis span.
fn has_closing_single_asterisk(bytes: &[u8], open_index: usize) -> bool {
    closing_single_asterisk(bytes, open_index).is_some()
}

/// Find the closing single asterisk for one emphasis span.
fn closing_single_asterisk(bytes: &[u8], open_index: usize) -> Option<usize> {
    if open_index + 1 >= bytes.len() || bytes[open_index + 1].is_ascii_whitespace() {
        return None;
    }

    let mut index = open_index + 1;

    while index < bytes.len() {
        if bytes[index] == b'`' {
            index += 1;
            while index < bytes.len() && bytes[index] != b'`' {
                index += 1;
            }
            index += usize::from(index < bytes.len());
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'*' && bytes[index + 1] == b'*' {
            index += 2;
            continue;
        }

        if bytes[index] == b'*' && index > open_index + 1 && !bytes[index - 1].is_ascii_whitespace()
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

/// Capitalize the first ASCII lowercase letter.
pub(super) fn capitalize_first(text: &str) -> Cow<'_, str> {
    if text.is_empty()
        || text.starts_with('`')
        || text
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || text
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        return Cow::Borrowed(text);
    }

    let mut prefix_len = 0;
    let mut rest = text;

    while let Some(stripped) = rest.strip_prefix("- ") {
        prefix_len += 2;
        rest = stripped;
    }

    if prefix_len > 0 {
        let capitalized = capitalize_first(rest);

        if matches!(capitalized, Cow::Borrowed(_)) {
            return Cow::Borrowed(text);
        }

        let mut result = String::with_capacity(prefix_len + capitalized.len());
        result.push_str(&text[..prefix_len]);
        result.push_str(&capitalized);

        return Cow::Owned(result);
    }

    let mut chars = text.chars();

    match chars.next() {
        Some(ch) if ch.is_ascii_lowercase() => {
            let mut result = String::with_capacity(text.len());
            result.push(ch.to_ascii_uppercase());
            result.push_str(chars.as_str());

            Cow::Owned(result)
        }
        _ => Cow::Borrowed(text),
    }
}

/// Append a trailing dot if the last character is word-like.
pub(super) fn append_trailing_dot(text: &str) -> Cow<'_, str> {
    if let Some(last_char) = text.chars().next_back()
        && (last_char.is_alphabetic() || last_char.is_ascii_digit() || last_char == '_')
    {
        let mut result = String::with_capacity(text.len() + 1);
        result.push_str(text);
        result.push('.');

        return Cow::Owned(result);
    }

    Cow::Borrowed(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_tag_kind() {
        assert_eq!(normalize_tag_kind("return"), "returns");
        assert_eq!(normalize_tag_kind("arg"), "param");
        assert_eq!(normalize_tag_kind("argument"), "param");
        assert_eq!(normalize_tag_kind("yield"), "yields");
        assert_eq!(normalize_tag_kind("prop"), "property");
        assert_eq!(normalize_tag_kind("memberOf"), "memberOf");
        assert_eq!(normalize_tag_kind("param"), "param");
        assert_eq!(normalize_tag_kind("returns"), "returns");
        assert_eq!(normalize_tag_kind("custom"), "custom");
    }

    #[test]
    fn test_normalize_markdown_emphasis() {
        assert_eq!(normalize_markdown_emphasis("__bold__"), "**bold**");
        assert_eq!(normalize_markdown_emphasis("*italic*"), "_italic_");
        assert_eq!(normalize_markdown_emphasis("`*code*`"), "`*code*`");
        assert_eq!(
            normalize_markdown_emphasis("__bold__ and *italic*"),
            "**bold** and _italic_"
        );
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("Hello"), "Hello");
        assert_eq!(capitalize_first("`code`"), "`code`");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("123"), "123");
        assert_eq!(capitalize_first("a"), "A");
        assert_eq!(capitalize_first("- hello"), "- Hello");
        assert_eq!(capitalize_first("- Hello"), "- Hello");
        assert_eq!(capitalize_first("- `code`"), "- `code`");
        assert_eq!(capitalize_first("http://example.com"), "http://example.com");
        assert_eq!(capitalize_first("HTTP://example.com"), "HTTP://example.com");
        assert_eq!(capitalize_first("Http://example.com"), "Http://example.com");
        assert_eq!(
            capitalize_first("https://example.com"),
            "https://example.com"
        );
        assert_eq!(
            capitalize_first("HTTPS://example.com"),
            "HTTPS://example.com"
        );
        assert_eq!(
            capitalize_first("Https://example.com"),
            "Https://example.com"
        );
    }

    #[test]
    fn test_append_trailing_dot() {
        assert_eq!(append_trailing_dot("hello"), "hello.");
        assert_eq!(append_trailing_dot("hello."), "hello.");
        assert_eq!(append_trailing_dot("`code`"), "`code`");
        assert_eq!(append_trailing_dot("value_"), "value_.");
    }
}
