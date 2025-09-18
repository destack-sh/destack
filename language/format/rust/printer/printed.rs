use dyst_language_source::{SourceId, Span};

use crate::SourceMarker;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Printed {
    code: String,
    range: Option<Span>,
    sourcemap: Vec<SourceMarker>,
    verbatim_ranges: Vec<Span>,
}

impl Printed {
    pub fn new(
        code: String,
        range: Option<Span>,
        sourcemap: Vec<SourceMarker>,
        verbatim_source: Vec<Span>,
    ) -> Self {
        Self {
            code,
            range,
            sourcemap,
            verbatim_ranges: verbatim_source,
        }
    }

    /// Construct an empty formatter result
    pub fn new_empty() -> Self {
        Self {
            code: String::new(),
            range: None,
            sourcemap: Vec::new(),
            verbatim_ranges: Vec::new(),
        }
    }

    /// Range of the input source file covered by this formatted code,
    /// or None if the entire file is covered in this instance
    pub fn range(&self) -> Option<Span> {
        self.range
    }

    /// Returns a list of [`SourceMarker`] mapping byte positions
    /// in the output string to the input source code.
    /// It's not guaranteed that the markers are sorted by source position.
    pub fn sourcemap(&self) -> &[SourceMarker] {
        &self.sourcemap
    }

    /// Returns a list of [`SourceMarker`] mapping byte positions
    /// in the output string to the input source code, consuming the result
    pub fn into_sourcemap(self) -> Vec<SourceMarker> {
        self.sourcemap
    }

    /// Takes the list of [`SourceMarker`] mapping byte positions in the output string
    /// to the input source code.
    pub fn take_sourcemap(&mut self) -> Vec<SourceMarker> {
        std::mem::take(&mut self.sourcemap)
    }

    /// Access the resulting code, borrowing the result
    pub fn as_code(&self) -> &str {
        &self.code
    }

    /// Access the resulting code, consuming the result
    pub fn into_code(self) -> String {
        self.code
    }

    /// The text in the formatted code that has been formatted as verbatim.
    pub fn verbatim(&self) -> impl Iterator<Item = (Span, &str)> {
        self.verbatim_ranges
            .iter()
            .map(|range| (*range, &self.code[range.start as usize..range.end as usize]))
    }

    /// Ranges of the formatted code that have been formatted as verbatim.
    pub fn verbatim_ranges(&self) -> &[Span] {
        &self.verbatim_ranges
    }

    /// Takes the ranges of nodes that have been formatted as verbatim, replacing them with an empty list.
    pub fn take_verbatim_ranges(&mut self) -> Vec<Span> {
        std::mem::take(&mut self.verbatim_ranges)
    }

    /// Slices the formatted code to the sub-slices that covers the passed `source_span` in `source`.
    ///
    /// The implementation uses the source map generated during formatting to find the closest range
    /// in the formatted document that covers `source_span` or more. The returned slice
    /// matches the `source_span` exactly (except indent, see below) if the formatter emits [`FormatElement::SourcePosition`] for
    /// the range's offsets.
    ///
    /// ## Indentation
    /// The indentation before `source_span.start` is replaced with the indentation returned by the formatter
    /// to fix up incorrectly intended code.
    ///
    /// Returns the entire document if the source map is empty.
    ///
    /// # Panics
    /// If `source_span` points to offsets that are not in the bounds of `source`.
    #[must_use]
    pub fn slice_range(self, source_span: Span, source: &str) -> PrintedSpan {
        let mut start_marker: Option<SourceMarker> = None;
        let mut end_marker: Option<SourceMarker> = None;

        // Note: The printer can generate multiple source map entries for the same source position.
        // For example if you have:
        // * token("a + b")
        // * `source_position(276)`
        // * `token(")")`
        // * `source_position(276)`
        // *  `hard_line_break`
        // The printer uses the source position 276 for both the tokens `)` and the `\n` because
        // there were multiple `source_position` entries in the IR with the same offset.
        // This can happen if multiple nodes start or end at the same position. A common example
        // for this are expressions and expression statement that always end at the same offset.
        //
        // Warning: Source markers are often emitted sorted by their source position but it's not guaranteed
        // and depends on the emitted `IR`.
        // They are only guaranteed to be sorted in increasing order by their destination position.
        for marker in self.sourcemap {
            // Take the closest start marker, but skip over start_markers that have the same start.
            if marker.source <= source_span.start
                && start_marker.is_none_or(|existing| existing.source < marker.source)
            {
                start_marker = Some(marker);
            }

            if marker.source >= source_span.end
                && end_marker.is_none_or(|existing| existing.source > marker.source)
            {
                end_marker = Some(marker);
            }
        }

        let (source_start, formatted_start) = start_marker
            .map(|marker| (marker.source, marker.dest))
            .unwrap_or_default();

        let (source_end, formatted_end) = end_marker
            .map_or((source.len(), self.code.len()), |marker| {
                (marker.source, marker.dest)
            });

        let source_span = Span::new(source_span.source, source_start, source_end);
        let formatted_span = Span::new(source_span.source, formatted_start, formatted_end);

        // Extend both ranges to include the indentation
        let source_span = extend_range_to_include_indent(source_span, source);
        let formatted_span = extend_range_to_include_indent(formatted_span, &self.code);

        PrintedSpan {
            code: self.code[formatted_span.start as usize..formatted_span.end as usize].to_string(),
            source_span,
        }
    }
}

/// Extends `range` backwards (by reducing `range.start`) to include any directly preceding whitespace (`\t` or ` `).
///
/// # Panics
/// If `range.start` is out of `source`'s bounds.
fn extend_range_to_include_indent(range: Span, source: &str) -> Span {
    let whitespace_len: usize = source[..range.start as usize]
        .chars()
        .rev()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .map(TextLen::text_len)
        .sum();

    Span::new(range.source, range.start - whitespace_len, range.end)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrintedSpan {
    code: String,
    source_span: Span,
}

impl PrintedSpan {
    pub fn new(code: String, source_span: Span) -> Self {
        Self { code, source_span }
    }

    pub fn empty() -> Self {
        Self {
            code: String::new(),
            source_span: Span::empty(SourceId::new(0)),
        }
    }

    /// The formatted code.
    pub fn as_code(&self) -> &str {
        &self.code
    }

    pub fn into_code(self) -> String {
        self.code
    }

    /// The range the formatted code corresponds to in the source document.
    pub fn source_span(&self) -> Span {
        self.source_span
    }
}
