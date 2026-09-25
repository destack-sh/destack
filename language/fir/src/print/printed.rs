use tspp_source::{FileId, Span};

use crate::format::FileMarker;

/// The result of printing with the printer.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Printed {
    code: String,
    range: Option<Span>,
    sourcemap: Vec<FileMarker>,
    verbatim_ranges: Vec<Span>,
}

impl Printed {
    /// Create a new Printed result with the given code, range, sourcemap, and verbatim ranges.
    pub fn new(
        code: String,
        range: Option<Span>,
        sourcemap: Vec<FileMarker>,
        verbatim_source: Vec<Span>,
    ) -> Self {
        Self {
            code,
            range,
            sourcemap,
            verbatim_ranges: verbatim_source,
        }
    }

    /// Create an empty formatter result.
    pub fn new_empty() -> Self {
        Self {
            code: String::new(),
            range: None,
            sourcemap: Vec::new(),
            verbatim_ranges: Vec::new(),
        }
    }

    /// Get the range of the input source file covered by this formatted code.
    /// Returns None if the entire file is covered in this instance.
    pub fn range(&self) -> Option<Span> {
        self.range
    }

    /// Get a list of FileMarkers mapping byte positions in the output string to the input source code.
    /// It's not guaranteed that the markers are sorted by source position.
    pub fn sourcemap(&self) -> &[FileMarker] {
        &self.sourcemap
    }

    /// Take the list of FileMarkers mapping byte positions in the output string to the input source code.
    pub fn into_sourcemap(self) -> Vec<FileMarker> {
        self.sourcemap
    }

    /// Take the list of FileMarkers mapping byte positions in the output string to the input source code.
    pub fn take_sourcemap(&mut self) -> Vec<FileMarker> {
        std::mem::take(&mut self.sourcemap)
    }

    /// Get the resulting code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.code
    }

    /// Take the resulting code, consuming the result.
    pub fn into_str(self) -> String {
        self.code
    }

    /// Get the text in the formatted code that has been formatted as verbatim.
    pub fn verbatim(&self) -> impl Iterator<Item = (Span, &str)> {
        self.verbatim_ranges
            .iter()
            .map(|range| (*range, &self.code[range.start as usize..range.end as usize]))
    }

    /// Get ranges of the formatted code that have been formatted as verbatim.
    pub fn verbatim_ranges(&self) -> &[Span] {
        &self.verbatim_ranges
    }

    /// Take the ranges of nodes that have been formatted as verbatim, replacing them with an empty list.
    pub fn take_verbatim_ranges(&mut self) -> Vec<Span> {
        std::mem::take(&mut self.verbatim_ranges)
    }

    /// Slice the formatted code to the sub-slices that covers the passed `source_span` in `source`.
    ///
    /// The implementation uses the source map generated during formatting to find the closest range
    /// in the formatted document that covers `source_span` or more.
    /// The returned slice matches the `source_span` exactly (except indent, see below) if the formatter emits FormatElement::FilePosition for the range's offsets.
    ///
    /// ## Indentation
    /// The indentation before `source_span.start` is replaced with the indentation returned by the formatter to fix up incorrectly intended code.
    ///
    /// Returns the entire document if the source map is empty.
    ///
    /// # Panics
    /// If `source_span` points to offsets that are not in the bounds of `source`.
    #[must_use]
    pub fn slice_range(self, source_span: Span, source: &str) -> PrintedSpan {
        let mut start_marker: Option<FileMarker> = None;
        let mut end_marker: Option<FileMarker> = None;

        // NOTE: The printer can generate multiple source map entries for the same source position
        // For example if you have:
        // * token("a + b")
        // * `source_position(276)`
        // * `token(")")`
        // * `source_position(276)`
        // *  `hard_line_break`
        // The printer uses the source position 276 for both the tokens `)` and the `\n` because
        // there were multiple `source_position` entries in the IR with the same offset.
        // This can happen if multiple nodes start or end at the same position.
        // A common example for this are expressions and expression statement that always end at the same offset.
        //
        // NOTE: File markers are often emitted sorted by their source position but it's not guaranteed
        // and depends on the emitted `IR`.
        // They are only guaranteed to be sorted in increasing order by their target position.
        for marker in self.sourcemap {
            // take the closest start marker, but skip over start_markers that have the same start
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
            .map_or((source.len() as u32, self.code.len() as u32), |marker| {
                (marker.source, marker.dest)
            });

        let source_span = Span::new(source_span.file, source_start, source_end);
        let formatted_span = Span::new(source_span.file, formatted_start, formatted_end);

        // extend both ranges to include the indentation
        let source_span = extend_range_to_include_indent(source_span, source);
        let formatted_span = extend_range_to_include_indent(formatted_span, &self.code);

        PrintedSpan {
            code: self.code[formatted_span.start as usize..formatted_span.end as usize].to_string(),
            source_span,
        }
    }
}

/// Extend `range` backwards (by reducing `range.start`) to include any directly preceding whitespace (`\t` or ` `).
///
/// # Panics
/// If `range.start` is out of `source`'s bounds.
fn extend_range_to_include_indent(range: Span, source: &str) -> Span {
    let whitespace_len: u32 = source[..range.start as usize]
        .chars()
        .rev()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .map(|character| character.len_utf8() as u32)
        .sum();

    Span::new(range.file, range.start - whitespace_len, range.end)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrintedSpan {
    code: String,
    source_span: Span,
}

impl PrintedSpan {
    /// Create a new PrintedSpan with the given code and source span.
    pub fn new(code: String, source_span: Span) -> Self {
        Self { code, source_span }
    }

    /// Create an empty PrintedSpan.
    pub fn empty() -> Self {
        Self {
            code: String::new(),
            source_span: Span::empty(FileId::new(0)),
        }
    }

    /// Get the formatted code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.code
    }

    /// Take the formatted code, consuming the result.
    pub fn into_str(self) -> String {
        self.code
    }

    /// Get the range the formatted code corresponds to in the source document.
    pub fn source_span(&self) -> Span {
        self.source_span
    }
}
