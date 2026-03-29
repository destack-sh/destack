use destack_fir::format::FormatOptions;
use destack_fir::print::PrintOptions;
use destack_source::{IndentStyle, LineEnding};

/// CSS format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssFormatOptions {
    /// The line ending to apply to formatted output.
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// The indent width.
    pub indent_width: u8,
    /// The target line width.
    pub line_width: u8,
}

impl Default for CssFormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl CssFormatOptions {
    /// Return pretty CSS format options.
    pub fn pretty() -> Self {
        Self::default()
    }

    /// Set the line ending.
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

    /// Set the target line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }
}

impl FormatOptions for CssFormatOptions {
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
            trim_trailing_whitespace: true,
        }
    }
}
