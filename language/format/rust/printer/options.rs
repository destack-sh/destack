use crate::IndentStyle;

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
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum LineEnding {
    /// Line Feed only (\n), common on Linux and macOS as well as inside git repos.
    #[default]
    LineFeed,
    /// Carriage Return + Line Feed characters (\r\n), common on Windows.
    CarriageReturnLineFeed,
    /// Carriage Return character only (\r), used very rarely.
    CarriageReturn,
}

impl LineEnding {
    /// Get the string representation of this line ending.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "\n",
            LineEnding::CarriageReturnLineFeed => "\r\n",
            LineEnding::CarriageReturn => "\r",
        }
    }

    /// Get the string used to configure this line ending.
    ///
    /// See [`LineEnding::as_str`] for the actual string representation of the line ending.
    #[inline]
    pub const fn as_setting_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "lf",
            LineEnding::CarriageReturnLineFeed => "crlf",
            LineEnding::CarriageReturn => "cr",
        }
    }
}
