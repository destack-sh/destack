use napi_derive::napi;
use dyst_source;

/// The indent style.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    /// Use tabs to indent.
    Tab,
    /// Use spaces to indent.
    Space,
}

impl From<IndentStyle> for dyst_source::IndentStyle {
    fn from(style: IndentStyle) -> Self {
        match style {
            IndentStyle::Tab => dyst_source::IndentStyle::Tab,
            IndentStyle::Space => dyst_source::IndentStyle::Space,
        }
    }
}

/// The type of line ending to apply to the printed input.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// Line feed only (\n), common on Linux and macOS as well as inside git repos.
    LineFeed,
    /// Carriage return + line feed characters (\r\n), common on Windows.
    CarriageReturnLineFeed,
    /// Carriage return character only (\r), used very rarely.
    CarriageReturn,
}

impl From<LineEnding> for dyst_source::LineEnding {
    fn from(ending: LineEnding) -> Self {
        match ending {
            LineEnding::LineFeed => dyst_source::LineEnding::LineFeed,
            LineEnding::CarriageReturnLineFeed => dyst_source::LineEnding::CarriageReturnLineFeed,
            LineEnding::CarriageReturn => dyst_source::LineEnding::CarriageReturn,
        }
    }
}

