use std::collections::HashSet;

use destack_dir as dir;
use destack_source::{EnclosingSpan, File, Span};

use crate::ModuleQueryContext;

/// Resolve the line start at the given offset.
pub(crate) fn offset_line_start(source: &str, offset: usize) -> usize {
    // clamp offset within the source bounds
    let offset = offset.min(source.len());
    let before = &source[..offset];

    before
        .rfind('\n')
        .map(|newline_index| newline_index + 1)
        .unwrap_or(0)
}

/// Extract the string literal prefix before a cursor offset.
pub(crate) fn extract_string_literal_prefix(source: &str, span: Span, offset: u32) -> String {
    // clamp the span within the source
    let start = span.start as usize;
    let end = span.end.min(source.len() as u32) as usize;
    if start >= end {
        return String::new();
    }

    // extract the literal text and cursor position
    let literal = &source[start..end];
    let offset_in_literal = offset.saturating_sub(span.start) as usize;

    // compute the content boundaries inside quotes
    let (content_start, content_end) = match literal.as_bytes().first().copied() {
        Some(b'"') | Some(b'\'') => {
            let end = literal
                .as_bytes()
                .last()
                .copied()
                .filter(|b| *b == b'"' || *b == b'\'')
                .map(|_| literal.len().saturating_sub(1))
                .unwrap_or(literal.len());
            (1, end)
        }
        _ => (0, literal.len()),
    };

    if offset_in_literal <= content_start {
        return String::new();
    }

    // return the prefix up to the cursor
    let prefix_end = offset_in_literal.min(content_end);
    literal[content_start..prefix_end].to_string()
}

/// Find the span for a string literal matching the provided text inside an enclosing span.
pub(crate) fn string_literal_span_in_enclosing(
    file: &File,
    tokens: &[dir::TokenSpan],
    enclosing: Span,
    target_text: &str,
) -> Option<Span> {
    // search string literal tokens inside the enclosing span
    for token in tokens {
        if !token_is_string_literal_in_span(token, enclosing) {
            continue;
        }

        // compare decoded literal contents
        let literal = file.span_str(token.span);
        let value = string_literal_value(literal)?;
        if value == target_text {
            return Some(token.span);
        }
    }

    None
}

/// Return whether one token is a string literal inside a span.
fn token_is_string_literal_in_span(token: &dir::TokenSpan, enclosing: Span) -> bool {
    token.span.file == enclosing.file
        && token.span.start >= enclosing.start
        && token.span.end <= enclosing.end
        && token.token.ty() == dir::TokenType::Literal
        && matches!(
            token.token.literal(),
            Some(dir::TokenLiteral::String { .. })
        )
}

/// Extract the string literal contents without quotes.
fn string_literal_value(literal: &str) -> Option<&str> {
    let bytes = literal.as_bytes();

    // require surrounding quote bytes
    if bytes.len() < 2 {
        return None;
    }

    // read the opening quote kind
    let quote = match bytes[0] {
        b'"' | b'\'' => bytes[0],
        _ => return None,
    };

    // require a matching trailing quote
    if bytes[bytes.len() - 1] != quote {
        return None;
    }

    Some(&literal[1..literal.len() - 1])
}

impl ModuleQueryContext<'_> {
    /// Return the span of a node.
    pub(crate) fn get_span(&self, view: dir::View<'_>, node_id: dir::LocalNodeIdAny) -> Span {
        let source_id = view.get_source_any(node_id);
        let source_span = self.source_index().get(source_id);

        Span::new(self.file_id(), source_span.start, source_span.end)
    }

    /// Return the main span of a node.
    pub(crate) fn get_main_span(&self, view: dir::View<'_>, node_id: dir::LocalNodeIdAny) -> Span {
        let source_id = view.get_source_any(node_id);
        let source_span = self
            .source_index()
            .get_main(source_id)
            .unwrap_or_else(|| panic!("missing main source span for source node {source_id}"));

        Span::new(self.file_id(), source_span.start, source_span.end)
    }

    /// Collect enclosing spans and sort from innermost to outermost.
    pub(crate) fn sorted_enclosing_spans(&self, start: u32, end: u32) -> Vec<EnclosingSpan> {
        let mut enclosing = self
            .source_index()
            .get_enclosing_spans(self.file_id(), start, end);

        enclosing.sort_by_key(|span| span.length);

        enclosing
    }

    /// Collect and sort enclosing spans for a set of probe offsets.
    pub(crate) fn enclosing_spans_at_offsets(
        &self,
        offsets: impl IntoIterator<Item = u32>,
    ) -> Vec<EnclosingSpan> {
        let mut enclosing = Vec::new();
        let mut seen = HashSet::new();

        // gather the enclosing spans for each probe offset
        for offset in offsets {
            let spans = self
                .source_index()
                .get_enclosing_spans(self.file_id(), offset, offset);
            for span in spans {
                if seen.insert(span.source_id) {
                    enclosing.push(span);
                }
            }
        }

        enclosing.sort_by_key(|span| span.length);

        enclosing
    }

    /// Collect enclosing spans at the cursor and previous byte.
    pub(crate) fn enclosing_spans_with_previous(&self, offset: u32) -> Vec<EnclosingSpan> {
        let mut offsets = vec![offset];
        if offset > 0 {
            offsets.push(offset - 1);
        }

        self.enclosing_spans_at_offsets(offsets)
    }
}

/// Sort spans by file and position and remove duplicates.
pub(crate) fn sort_and_dedup_spans(spans: &mut Vec<Span>) {
    // sort spans for stable ordering and deduplication
    spans.sort_by(|left, right| {
        (left.file.0, left.start, left.end).cmp(&(right.file.0, right.start, right.end))
    });

    // remove exact duplicate spans
    spans.dedup_by(|left, right| {
        left.file == right.file && left.start == right.start && left.end == right.end
    });
}
