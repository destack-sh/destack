/// The formatting options.
#[derive(Debug, Copy, Clone, Default)]
pub struct FormattingOptions {
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl FormattingOptions {
    /// The default formatting options.
    pub const DEFAULT: Self = Self {
        line_ending: LineEnding::LineFeed,
        indent_style: IndentStyle::Space,
        indent_width: 4,
        line_width: 100,
    };

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

/// The indent style.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, Default)]
pub enum IndentStyle {
    /// Use tabs to indent.
    #[default]
    Tab,
    /// Use spaces to indent.
    Space,
}

impl IndentStyle {
    /// Check if this is an [`IndentStyle::Tab`].
    pub const fn is_tab(&self) -> bool {
        matches!(self, IndentStyle::Tab)
    }

    /// Check if this is an [`IndentStyle::Space`].
    pub const fn is_space(&self) -> bool {
        matches!(self, IndentStyle::Space)
    }

    /// Get the string representation of the indent style.
    pub const fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Tab => "tab",
            IndentStyle::Space => "space",
        }
    }
}

impl std::fmt::Display for IndentStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The type of line ending to apply to the printed input.
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
