use std::num::NonZeroU16;

use crate::IndentStyle;

#[derive(Debug, Copy, Clone, Default)]
pub struct PrintOptions {
    /// The type of line ending to apply to the printed input
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// Spaces per indent.
    pub indent_width: NonZeroU16 = NonZeroU16::new(4).unwrap(),
    /// Maximum line length (best effort).
    pub line_width: NonZeroU16 = NonZeroU16::new(100).unwrap(),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum LineEnding {
    ///  Line Feed only (\n), common on Linux and macOS as well as inside git repos
    #[default]
    LineFeed,

    /// Carriage Return + Line Feed characters (\r\n), common on Windows
    CarriageReturnLineFeed,

    /// Carriage Return character only (\r), used very rarely
    CarriageReturn,
}

impl LineEnding {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            LineEnding::LineFeed => "\n",
            LineEnding::CarriageReturnLineFeed => "\r\n",
            LineEnding::CarriageReturn => "\r",
        }
    }

    /// Returns the string used to configure this line ending.
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
