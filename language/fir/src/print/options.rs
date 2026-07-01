use crate::format::{IndentStyle, LineEnding};

/// The largest printable output offset representable by FIR source markers.
pub const MAX_OUTPUT_BYTES: u32 = u32::MAX;

#[derive(Debug, Copy, Clone, Default)]
pub struct PrintOptions {
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
    /// Remove trailing spaces and tabs before each emitted newline.
    pub trim_trailing_whitespace: bool = false,
    /// Maximum formatted output bytes to emit.
    pub max_output_bytes: u32 = MAX_OUTPUT_BYTES,
}

impl PrintOptions {
    /// Set the line ending type.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Set whether to trim trailing spaces and tabs before newlines.
    pub fn with_trim_trailing_whitespace(mut self, trim_trailing_whitespace: bool) -> Self {
        self.trim_trailing_whitespace = trim_trailing_whitespace;
        self
    }

    /// Set the maximum formatted output bytes to emit.
    pub fn with_max_output_bytes(mut self, max_output_bytes: u32) -> Self {
        self.max_output_bytes = max_output_bytes;
        self
    }
}
