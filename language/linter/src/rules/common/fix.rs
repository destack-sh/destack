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
