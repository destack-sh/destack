use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// The indent style.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LineEnding {
    /// Line Feed only (\n), common on Linux and macOS as well as inside git repos.
    #[default]
    #[serde(rename = "lf")]
    LineFeed,
    /// Carriage Return + Line Feed characters (\r\n), common on Windows.
    #[serde(rename = "crlf")]
    CarriageReturnLineFeed,
    /// Carriage Return character only (\r), used very rarely.
    #[serde(rename = "cr")]
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
