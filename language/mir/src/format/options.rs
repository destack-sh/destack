use tspp_fir::format::FormatOptions as FirFormatOptions;
use tspp_fir::print::{MAX_OUTPUT_BYTES, PrintOptions};
use tspp_source::{IndentStyle, LineEnding};

/// MIR formatting options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormatOptions {
    /// Line ending style.
    pub line_ending: LineEnding,
    /// Indent style.
    pub indent_style: IndentStyle,
    /// Indent width.
    pub indent_width: u8,
    /// Maximum line width.
    pub line_width: u8,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl FormatOptions {
    /// Convert these options into FIR print options.
    fn print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: false,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }
}

impl FirFormatOptions for FormatOptions {
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
        self.print_options()
    }
}
