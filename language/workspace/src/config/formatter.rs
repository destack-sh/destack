use destack_source::{IndentStyle, LineEnding};

/// Quote style for string literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
                // Single char (not empty) uses single quotes
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// Formatter options.
///
/// Controls code style decisions made by the formatter.
/// Default values match Prettier's defaults for familiarity.
#[derive(Debug, Copy, Clone)]
pub struct FormatterOptions {
    /// Line ending style (LF, CRLF, CR).
    pub line_ending: LineEnding,
    /// Indent with spaces or tabs.
    pub indent_style: IndentStyle,
    /// Number of spaces per indent level (when using spaces).
    pub indent_width: u8,
    /// Target line width (best effort, not a hard limit).
    pub line_width: u16,

    /// Quote style for string literals.
    pub quote_style: QuoteStyle,
    /// Trailing comma policy for multi-line constructs.
    pub trailing_comma: TrailingComma,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false).
    pub bracket_spacing: bool,
    /// Arrow function parentheses policy.
    pub arrow_parens: ArrowParentheses,
    /// Object property quoting policy.
    pub quote_props: QuoteProperty,

    /// Put `>` of multi-line tree/JSX on same line as last attribute.
    pub bracket_same_line: bool,
    /// Force each tree/JSX attribute onto its own line.
    pub single_attribute_per_line: bool,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatterOptions {
    /// Create options with default values matching Prettier.
    pub fn new() -> Self {
        Self {
            // layout
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
            // syntax
            quote_style: QuoteStyle::Semantic,
            trailing_comma: TrailingComma::All,
            bracket_spacing: true,
            arrow_parens: ArrowParentheses::Always,
            quote_props: QuoteProperty::AsNeeded,
            // tree/jsx
            bracket_same_line: false,
            single_attribute_per_line: false,
        }
    }

    /// Create options with tab indentation for testing.
    #[cfg(test)]
    pub fn default_tab() -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            ..Self::new()
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

    /// Set the quote style.
    pub fn with_quote_style(mut self, quote_style: QuoteStyle) -> Self {
        self.quote_style = quote_style;
        self
    }

    /// Set the trailing comma policy.
    pub fn with_trailing_comma(mut self, trailing_comma: TrailingComma) -> Self {
        self.trailing_comma = trailing_comma;
        self
    }

    /// Set bracket spacing.
    pub fn with_bracket_spacing(mut self, bracket_spacing: bool) -> Self {
        self.bracket_spacing = bracket_spacing;
        self
    }

    /// Set arrow function parentheses policy.
    pub fn with_arrow_parens(mut self, arrow_parens: ArrowParentheses) -> Self {
        self.arrow_parens = arrow_parens;
        self
    }

    /// Set object property quoting policy.
    pub fn with_quote_props(mut self, quote_props: QuoteProperty) -> Self {
        self.quote_props = quote_props;
        self
    }

    /// Set bracket same line policy.
    pub fn with_bracket_same_line(mut self, bracket_same_line: bool) -> Self {
        self.bracket_same_line = bracket_same_line;
        self
    }

    /// Set single attribute per line policy.
    pub fn with_single_attribute_per_line(mut self, single_attribute_per_line: bool) -> Self {
        self.single_attribute_per_line = single_attribute_per_line;
        self
    }
}
