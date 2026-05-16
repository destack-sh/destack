use serde::{Deserialize, Serialize};

use destack_source::{IndentStyle, LineEnding};

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

impl std::fmt::Display for QuoteStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl std::fmt::Display for TrailingComma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl std::fmt::Display for ArrowParentheses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl std::fmt::Display for QuoteProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl std::fmt::Display for OrganizeImports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl std::fmt::Display for ImportSortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How to choose between single-line and multiline JSDoc comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum JsdocCommentLineStrategy {
    /// Use one line when the content fits on one line.
    #[default]
    SingleLine,
    /// Always use multiline comment blocks.
    Multiline,
    /// Preserve an existing multiline block shape.
    Keep,
}

/// How to wrap JSDoc prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum JsdocLineWrappingStyle {
    /// Re-wrap text greedily to the configured width.
    #[default]
    Greedy,
    /// Preserve original line breaks when they fit.
    Balance,
}

/// JSDoc comment body formatting options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsdocOptions {
    /// Capitalize the first word of prose descriptions.
    pub capitalize_descriptions: bool,
    /// Comment block line strategy.
    pub comment_line_strategy: JsdocCommentLineStrategy,
    /// Separate groups of different tag kinds with blank lines.
    pub separate_tag_groups: bool,
    /// Separate returns tags from parameter tags.
    pub separate_returns_from_param: bool,
    /// Add a trailing dot to prose descriptions.
    pub description_with_dot: bool,
    /// Add default values to parameter descriptions.
    pub add_default_to_description: bool,
    /// Prefer fenced code blocks over indented code blocks.
    pub prefer_code_fences: bool,
    /// Prose wrapping style.
    pub line_wrapping_style: JsdocLineWrappingStyle,
    /// Emit descriptions as an explicit tag.
    pub description_tag: bool,
    /// Keep indentation in example code that cannot be parsed.
    pub keep_unparsable_example_indent: bool,
}

impl Default for JsdocOptions {
    fn default() -> Self {
        Self {
            capitalize_descriptions: true,
            comment_line_strategy: JsdocCommentLineStrategy::SingleLine,
            separate_tag_groups: false,
            separate_returns_from_param: false,
            description_with_dot: false,
            add_default_to_description: true,
            prefer_code_fences: false,
            line_wrapping_style: JsdocLineWrappingStyle::Greedy,
            description_tag: false,
            keep_unparsable_example_indent: false,
        }
    }
}

/// Formatter options.
///
/// Controls code style decisions made by the formatter.
/// Default values match the standard formatter defaults used by Destack.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    /// Create options with the standard Destack defaults.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_layout_options() {
        let input = r#"
            {
                "lineWidth": 120,
                "indentWidth": 2,
                "indentStyle": "tab",
                "lineEnding": "crlf"
            }
        "#;

        let options: FormatterOptions = serde_json::from_str(input).unwrap();

        assert_eq!(options.line_width, 120);
        assert_eq!(options.indent_width, 2);
        assert_eq!(options.indent_style, IndentStyle::Tab);
        assert_eq!(options.line_ending, LineEnding::CarriageReturnLineFeed);
    }
}
