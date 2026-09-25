use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use tspp_source::{IndentStyle, LineEnding};

/// Quote style for string literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum QuoteStyle {
    /// Use double quotes: `"hello"`.
    #[default]
    Double,
    /// Use single quotes: `'hello'`.
    Single,
    /// Use single quotes for single characters, double quotes for strings.
    /// `'a'` for characters, `"hello"` and `""` for strings.
    Semantic,
}

impl QuoteStyle {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "double" | "false" => Some(Self::Double),
            "single" | "true" => Some(Self::Single),
            "semantic" | "auto" => Some(Self::Semantic),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Double => "double",
            Self::Single => "single",
            Self::Semantic => "semantic",
        }
    }

    /// Get the quote character for a given content.
    /// For `Semantic`, returns single quote for single-char content, double otherwise.
    pub fn char_for(&self, content: &str) -> char {
        match self {
            Self::Double => '"',
            Self::Single => '\'',
            Self::Semantic => {
                if content.chars().count() == 1 {
                    '\''
                } else {
                    '"'
                }
            }
        }
    }

    /// Get the default quote character (for non-semantic contexts).
    pub fn char(&self) -> char {
        match self {
            Self::Double => '"',
            Self::Single => '\'',
            Self::Semantic => '"', // default to double for semantic
        }
    }
}

impl Display for QuoteStyle {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Trailing comma policy for multi-line constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TrailingComma {
    /// Add trailing commas everywhere valid in ES2017+ (functions, arrays, objects).
    #[default]
    All,
    /// Add trailing commas where valid in ES5 (arrays, objects, not function params).
    Es5,
    /// Never add trailing commas.
    None,
}

impl TrailingComma {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "all" => Some(Self::All),
            "es5" => Some(Self::Es5),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Es5 => "es5",
            Self::None => "none",
        }
    }

    /// Whether trailing commas are allowed in function parameters.
    pub fn in_functions(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Whether trailing commas are allowed in arrays and objects.
    pub fn in_collections(&self) -> bool {
        !matches!(self, Self::None)
    }
}

impl Display for TrailingComma {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Arrow function parentheses policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ArrowParentheses {
    /// Always include parentheses: `(x) => x`.
    #[default]
    Always,
    /// Omit parentheses when possible: `x => x`.
    Avoid,
}

impl ArrowParentheses {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "always" => Some(Self::Always),
            "avoid" => Some(Self::Avoid),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Avoid => "avoid",
        }
    }
}

impl Display for ArrowParentheses {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Object property quote style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum QuoteProperty {
    /// Only quote properties when required (e.g., `{ "foo-bar": 1, baz: 2 }`).
    #[default]
    AsNeeded,
    /// Quote all properties consistently if any require quotes.
    Consistent,
    /// Preserve the original quoting from source.
    Preserve,
}

impl QuoteProperty {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "as-needed" | "asneeded" => Some(Self::AsNeeded),
            "consistent" => Some(Self::Consistent),
            "preserve" => Some(Self::Preserve),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AsNeeded => "as-needed",
            Self::Consistent => "consistent",
            Self::Preserve => "preserve",
        }
    }
}

impl Display for QuoteProperty {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Import organization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OrganizeImports {
    /// Organize imports: sort statements by group and specifiers alphabetically.
    On,
    /// Don't reorder imports (preserve original order).
    #[default]
    Off,
}

impl OrganizeImports {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "on" | "true" => Some(Self::On),
            "off" | "false" => Some(Self::Off),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
        }
    }

    /// Whether import organization is enabled.
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::On)
    }
}

impl Display for OrganizeImports {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Sort order for import/export specifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ImportSortOrder {
    /// Natural sort: numbers ordered as integers (a1 < a2 < a10).
    #[default]
    Natural,
    /// Alphabetical/lexicographic sort (a1 < a10 < a2).
    Alphabetical,
}

impl ImportSortOrder {
    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "natural" => Some(Self::Natural),
            "alphabetical" | "lexicographic" => Some(Self::Alphabetical),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Alphabetical => "alphabetical",
        }
    }
}

impl Display for ImportSortOrder {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Formatter options.
///
/// Controls code style decisions made by the formatter.
/// Default values are the standard TS++ formatter defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct FormatterOptions {
    /// Line ending style (LF, CRLF, CR).
    pub line_ending: LineEnding,
    /// Indent with spaces or tabs.
    pub indent_style: IndentStyle,
    /// Number of spaces per indent level (when using spaces).
    pub indent_width: u8,
    /// Target line width (best effort, not a hard limit).
    pub line_width: u16,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatterOptions {
    /// Create options with the standard TS++ defaults.
    pub fn new() -> Self {
        Self {
            // layout
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }

    /// Set the line ending style.
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
    pub fn with_line_width(mut self, line_width: u16) -> Self {
        self.line_width = line_width;
        self
    }
}
