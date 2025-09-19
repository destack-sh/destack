use crate::format::{IndentStyle, LineEnding};
use crate::print::PrintOptions;

pub trait FormatOptions {
    /// Get the indent style.
    fn indent_style(&self) -> IndentStyle;

    /// Get the visual width of an indent.
    fn indent_width(&self) -> u8;

    /// Get the max width of a line.
    ///
    /// Defaults to 80.
    fn line_width(&self) -> u8;

    /// Derive the print options from these format options.
    fn as_print_options(&self) -> PrintOptions;
}

/// Simple format options (mostly for testing).
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct SimpleFormatOptions {
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl SimpleFormatOptions {
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }
}

impl FormatOptions for SimpleFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}
