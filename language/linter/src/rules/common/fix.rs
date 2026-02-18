use destack_source::Span;

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
