use serde::{Deserialize, Serialize};

/// The indent style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IndentStyle {
    /// Use tabs to indent.
    Tab,
    /// Use spaces to indent.
    Space,
}

impl From<IndentStyle> for destack_source::IndentStyle {
    fn from(style: IndentStyle) -> Self {
        match style {
            IndentStyle::Tab => destack_source::IndentStyle::Tab,
            IndentStyle::Space => destack_source::IndentStyle::Space,
        }
    }
}

/// The type of line ending to apply to the printed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineEnding {
    /// Line feed only (\n), common on Linux and macOS as well as inside git repos.
    LineFeed,
    /// Carriage return + line feed characters (\r\n), common on Windows.
    CarriageReturnLineFeed,
    /// Carriage return character only (\r), used very rarely.
    CarriageReturn,
}

impl From<LineEnding> for destack_source::LineEnding {
    fn from(ending: LineEnding) -> Self {
        match ending {
            LineEnding::LineFeed => destack_source::LineEnding::LineFeed,
            LineEnding::CarriageReturnLineFeed => {
                destack_source::LineEnding::CarriageReturnLineFeed
            }
            LineEnding::CarriageReturn => destack_source::LineEnding::CarriageReturn,
        }
    }
}
