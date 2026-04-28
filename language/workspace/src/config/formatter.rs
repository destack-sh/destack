use serde::Deserialize;

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

/// Import organization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy)]
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
    pub arrow_parentheses: ArrowParentheses,
    /// Object property quoting policy.
    pub quote_property: QuoteProperty,

    /// Put `>` of multi-line tree/JSX on same line as last attribute.
    pub bracket_same_line: bool,
    /// Force each tree/JSX attribute onto its own line.
    pub single_attribute_per_line: bool,

    /// Whether to organize/sort imports and exports.
    pub organize_imports: OrganizeImports,
    /// Sort order for import/export specifiers within `{ }`.
    pub import_sort_order: ImportSortOrder,
    /// JSDoc comment body formatting options.
    pub jsdoc: Option<JsdocOptions>,
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
            // syntax
            quote_style: QuoteStyle::Semantic,
            trailing_comma: TrailingComma::All,
            bracket_spacing: true,
            arrow_parentheses: ArrowParentheses::Always,
            quote_property: QuoteProperty::AsNeeded,
            // tree/jsx
            bracket_same_line: false,
            single_attribute_per_line: false,
            // imports
            organize_imports: OrganizeImports::Off,
            import_sort_order: ImportSortOrder::Natural,
            // comments
            jsdoc: None,
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
        self.arrow_parentheses = arrow_parens;
        self
    }

    /// Set object property quoting policy.
    pub fn with_quote_props(mut self, quote_props: QuoteProperty) -> Self {
        self.quote_property = quote_props;
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

    /// Set import organization mode.
    pub fn with_organize_imports(mut self, organize_imports: OrganizeImports) -> Self {
        self.organize_imports = organize_imports;
        self
    }

    /// Set import sort order.
    pub fn with_import_sort_order(mut self, import_sort_order: ImportSortOrder) -> Self {
        self.import_sort_order = import_sort_order;
        self
    }
}

/// Line ending style for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LineEndingJson {
    /// Unix-style line endings (LF).
    #[serde(rename = "lf")]
    Lf,
    /// Windows-style line endings (CRLF).
    #[serde(rename = "crlf")]
    Crlf,
    /// Classic Mac-style line endings (CR).
    #[serde(rename = "cr")]
    Cr,
}

impl From<LineEndingJson> for LineEnding {
    fn from(value: LineEndingJson) -> Self {
        match value {
            LineEndingJson::Lf => LineEnding::LineFeed,
            LineEndingJson::Crlf => LineEnding::CarriageReturnLineFeed,
            LineEndingJson::Cr => LineEnding::CarriageReturn,
        }
    }
}

/// Indent style for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum IndentStyleJson {
    #[serde(alias = "tabs")]
    Tab,
    #[serde(alias = "spaces")]
    Space,
}

impl From<IndentStyleJson> for IndentStyle {
    fn from(value: IndentStyleJson) -> Self {
        match value {
            IndentStyleJson::Tab => IndentStyle::Tab,
            IndentStyleJson::Space => IndentStyle::Space,
        }
    }
}

/// Quote style for JSON deserialization (`singleQuote`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum QuoteStyleJson {
    /// Use double quotes.
    Double,
    /// Use single quotes.
    Single,
    /// Use single quotes for single characters, double quotes for strings.
    #[serde(alias = "auto")]
    Semantic,
}

impl From<QuoteStyleJson> for QuoteStyle {
    fn from(value: QuoteStyleJson) -> Self {
        match value {
            QuoteStyleJson::Double => QuoteStyle::Double,
            QuoteStyleJson::Single => QuoteStyle::Single,
            QuoteStyleJson::Semantic => QuoteStyle::Semantic,
        }
    }
}

/// Trailing comma policy for JSON deserialization (`trailingComma`).
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TrailingCommaJson {
    /// Trailing commas everywhere valid.
    All,
    /// Trailing commas where valid in ES5.
    Es5,
    /// No trailing commas.
    None,
}

impl From<TrailingCommaJson> for TrailingComma {
    fn from(value: TrailingCommaJson) -> Self {
        match value {
            TrailingCommaJson::All => TrailingComma::All,
            TrailingCommaJson::Es5 => TrailingComma::Es5,
            TrailingCommaJson::None => TrailingComma::None,
        }
    }
}

/// Arrow function parentheses for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ArrowParenthesesJson {
    /// Always include parentheses.
    Always,
    /// Omit when possible.
    Avoid,
}

impl From<ArrowParenthesesJson> for ArrowParentheses {
    fn from(value: ArrowParenthesesJson) -> Self {
        match value {
            ArrowParenthesesJson::Always => ArrowParentheses::Always,
            ArrowParenthesesJson::Avoid => ArrowParentheses::Avoid,
        }
    }
}

/// Object property quoting for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum QuotePropertyJson {
    /// Only quote when required.
    AsNeeded,
    /// Quote all if any require quotes.
    Consistent,
    /// Preserve original quoting.
    Preserve,
}

impl From<QuotePropertyJson> for QuoteProperty {
    fn from(value: QuotePropertyJson) -> Self {
        match value {
            QuotePropertyJson::AsNeeded => QuoteProperty::AsNeeded,
            QuotePropertyJson::Consistent => QuoteProperty::Consistent,
            QuotePropertyJson::Preserve => QuoteProperty::Preserve,
        }
    }
}

/// Whether to organize imports.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OrganizeImportsJson {
    /// Organize imports: sort statements by group and specifiers alphabetically.
    On,
    /// Don't reorder imports (preserve original order).
    Off,
}

impl From<OrganizeImportsJson> for OrganizeImports {
    fn from(value: OrganizeImportsJson) -> Self {
        match value {
            OrganizeImportsJson::On => OrganizeImports::On,
            OrganizeImportsJson::Off => OrganizeImports::Off,
        }
    }
}

/// Sort order for import specifiers.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImportSortOrderJson {
    /// Natural sort: numbers ordered as integers (a1 < a2 < a10).
    Natural,
    /// Alphabetical/lexicographic sort (a1 < a10 < a2).
    Alphabetical,
}

impl From<ImportSortOrderJson> for ImportSortOrder {
    fn from(value: ImportSortOrderJson) -> Self {
        match value {
            ImportSortOrderJson::Natural => ImportSortOrder::Natural,
            ImportSortOrderJson::Alphabetical => ImportSortOrder::Alphabetical,
        }
    }
}

/// JSDoc comment block line strategy JSON value.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum JsdocCommentLineStrategyJson {
    /// Use one line when the content fits on one line.
    #[serde(alias = "single-line", alias = "single_line")]
    SingleLine,
    /// Always use multiline comment blocks.
    Multiline,
    /// Preserve an existing multiline block shape.
    Keep,
}

impl From<JsdocCommentLineStrategyJson> for JsdocCommentLineStrategy {
    fn from(value: JsdocCommentLineStrategyJson) -> Self {
        match value {
            JsdocCommentLineStrategyJson::SingleLine => JsdocCommentLineStrategy::SingleLine,
            JsdocCommentLineStrategyJson::Multiline => JsdocCommentLineStrategy::Multiline,
            JsdocCommentLineStrategyJson::Keep => JsdocCommentLineStrategy::Keep,
        }
    }
}

/// JSDoc prose wrapping JSON value.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum JsdocLineWrappingStyleJson {
    /// Re-wrap text greedily to the configured width.
    Greedy,
    /// Preserve original line breaks when they fit.
    Balance,
}

impl From<JsdocLineWrappingStyleJson> for JsdocLineWrappingStyle {
    fn from(value: JsdocLineWrappingStyleJson) -> Self {
        match value {
            JsdocLineWrappingStyleJson::Greedy => JsdocLineWrappingStyle::Greedy,
            JsdocLineWrappingStyleJson::Balance => JsdocLineWrappingStyle::Balance,
        }
    }
}

/// JSDoc formatter options JSON object.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct JsdocJson {
    /// Capitalize the first word of prose descriptions.
    #[serde(alias = "capitalize_descriptions")]
    pub capitalize_descriptions: Option<bool>,
    /// Comment block line strategy.
    #[serde(alias = "comment_line_strategy")]
    pub comment_line_strategy: Option<JsdocCommentLineStrategyJson>,
    /// Separate groups of different tag kinds with blank lines.
    #[serde(alias = "separate_tag_groups")]
    pub separate_tag_groups: Option<bool>,
    /// Separate returns tags from parameter tags.
    #[serde(alias = "separate_returns_from_param")]
    pub separate_returns_from_param: Option<bool>,
    /// Add a trailing dot to prose descriptions.
    #[serde(alias = "description_with_dot")]
    pub description_with_dot: Option<bool>,
    /// Add default values to parameter descriptions.
    #[serde(alias = "add_default_to_description")]
    pub add_default_to_description: Option<bool>,
    /// Prefer fenced code blocks over indented code blocks.
    #[serde(alias = "prefer_code_fences")]
    pub prefer_code_fences: Option<bool>,
    /// Prose wrapping style.
    #[serde(alias = "line_wrapping_style")]
    pub line_wrapping_style: Option<JsdocLineWrappingStyleJson>,
    /// Emit descriptions as an explicit tag.
    #[serde(alias = "description_tag")]
    pub description_tag: Option<bool>,
    /// Keep indentation in example code that cannot be parsed.
    #[serde(alias = "keep_unparsable_example_indent")]
    pub keep_unparsable_example_indent: Option<bool>,
}

impl JsdocJson {
    /// Apply JSDoc options to a JSDoc options struct.
    pub fn apply(&self, options: &mut JsdocOptions) {
        if let Some(capitalize_descriptions) = self.capitalize_descriptions {
            options.capitalize_descriptions = capitalize_descriptions;
        }
        if let Some(comment_line_strategy) = self.comment_line_strategy {
            options.comment_line_strategy = comment_line_strategy.into();
        }
        if let Some(separate_tag_groups) = self.separate_tag_groups {
            options.separate_tag_groups = separate_tag_groups;
        }
        if let Some(separate_returns_from_param) = self.separate_returns_from_param {
            options.separate_returns_from_param = separate_returns_from_param;
        }
        if let Some(description_with_dot) = self.description_with_dot {
            options.description_with_dot = description_with_dot;
        }
        if let Some(add_default_to_description) = self.add_default_to_description {
            options.add_default_to_description = add_default_to_description;
        }
        if let Some(prefer_code_fences) = self.prefer_code_fences {
            options.prefer_code_fences = prefer_code_fences;
        }
        if let Some(line_wrapping_style) = self.line_wrapping_style {
            options.line_wrapping_style = line_wrapping_style.into();
        }
        if let Some(description_tag) = self.description_tag {
            options.description_tag = description_tag;
        }
        if let Some(keep_unparsable_example_indent) = self.keep_unparsable_example_indent {
            options.keep_unparsable_example_indent = keep_unparsable_example_indent;
        }
    }
}

/// Formatter options (top-level, like Biome/Deno).
///
/// Field names use familiar formatter option naming.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FormatterJson {
    /// Line ending style: "lf", "crlf", or "cr".
    #[serde(alias = "endOfLine")]
    pub line_ending: Option<LineEndingJson>,
    /// Use tabs instead of spaces.
    #[serde(alias = "useTabs")]
    pub use_tabs: Option<bool>,
    /// Indent style: "tab" or "space".
    pub indent_style: Option<IndentStyleJson>,
    /// Number of spaces per indent. Default: 4.
    #[serde(alias = "tabWidth")]
    pub indent_width: Option<u8>,
    /// Maximum line width (best effort). Default: 100.
    #[serde(alias = "printWidth")]
    pub line_width: Option<u16>,

    /// Quote style: "double", "single", or "semantic".
    pub quote_style: Option<QuoteStyleJson>,
    /// Use single quotes. Takes precedence over quoteStyle.
    pub single_quote: Option<bool>,
    /// Trailing comma policy: "all", "es5", or "none".
    pub trailing_comma: Option<TrailingCommaJson>,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false). Default: true.
    pub bracket_spacing: Option<bool>,
    /// Arrow function parentheses: "always" or "avoid".
    pub arrow_parens: Option<ArrowParenthesesJson>,
    /// Object property quoting: "as-needed", "consistent", or "preserve".
    pub quote_props: Option<QuotePropertyJson>,

    /// Put `>` of multi-line JSX on same line as last attribute.
    #[serde(alias = "jsxBracketSameLine")]
    pub bracket_same_line: Option<bool>,
    /// Force each JSX attribute onto its own line.
    pub single_attribute_per_line: Option<bool>,

    /// Whether to organize imports: "on" or "off". Default: off.
    pub organize_imports: Option<OrganizeImportsJson>,
    /// Sort order for import specifiers: "natural" or "alphabetical". Default: natural.
    pub import_sort_order: Option<ImportSortOrderJson>,
    /// JSDoc comment body formatting options.
    pub jsdoc: Option<JsdocJson>,
}

impl FormatterJson {
    /// Apply formatter options to a FormatterOptions struct.
    pub fn apply(&self, options: &mut FormatterOptions) {
        // layout
        if let Some(line_ending) = self.line_ending {
            options.line_ending = line_ending.into();
        }
        if let Some(true) = self.use_tabs {
            options.indent_style = IndentStyle::Tab;
        } else if let Some(indent_style) = self.indent_style {
            options.indent_style = indent_style.into();
        }
        if let Some(indent_width) = self.indent_width {
            options.indent_width = indent_width;
        }
        if let Some(line_width) = self.line_width {
            options.line_width = line_width;
        }

        // syntax: singleQuote takes precedence over quoteStyle
        if let Some(single_quote) = self.single_quote {
            options.quote_style = if single_quote {
                QuoteStyle::Single
            } else {
                QuoteStyle::Double
            };
        } else if let Some(quote_style) = self.quote_style {
            options.quote_style = quote_style.into();
        }
        if let Some(trailing_comma) = self.trailing_comma {
            options.trailing_comma = trailing_comma.into();
        }
        if let Some(bracket_spacing) = self.bracket_spacing {
            options.bracket_spacing = bracket_spacing;
        }
        if let Some(arrow_parens) = self.arrow_parens {
            options.arrow_parentheses = arrow_parens.into();
        }
        if let Some(quote_props) = self.quote_props {
            options.quote_property = quote_props.into();
        }

        // tree/jsx
        if let Some(bracket_same_line) = self.bracket_same_line {
            options.bracket_same_line = bracket_same_line;
        }
        if let Some(single_attribute_per_line) = self.single_attribute_per_line {
            options.single_attribute_per_line = single_attribute_per_line;
        }

        // imports
        if let Some(organize_imports) = self.organize_imports {
            options.organize_imports = organize_imports.into();
        }
        if let Some(import_sort_order) = self.import_sort_order {
            options.import_sort_order = import_sort_order.into();
        }

        // comments
        if let Some(jsdoc) = self.jsdoc.as_ref() {
            let mut jsdoc_options = options.jsdoc.unwrap_or_default();
            jsdoc.apply(&mut jsdoc_options);
            options.jsdoc = Some(jsdoc_options);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jsdoc_options_with_camel_case_values() {
        let input = r#"
            {
                "jsdoc": {
                    "commentLineStrategy": "singleLine",
                    "lineWrappingStyle": "balance",
                    "separateTagGroups": true
                }
            }
        "#;

        // parse the full formatter JSON shape
        let json: FormatterJson = serde_json::from_str(input).unwrap();
        let mut options = FormatterOptions::default();
        json.apply(&mut options);

        let expected = JsdocOptions {
            comment_line_strategy: JsdocCommentLineStrategy::SingleLine,
            line_wrapping_style: JsdocLineWrappingStyle::Balance,
            separate_tag_groups: true,
            ..JsdocOptions::default()
        };

        assert_eq!(options.jsdoc, Some(expected));
    }
}
