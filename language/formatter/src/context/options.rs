use destack_fir::format::FormatOptions;
use destack_fir::print::PrintOptions;
use destack_source::{IndentStyle, LanguageType, LineEnding};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, ImportSortOrder, OrganizeImports, QuoteProperty,
    QuoteStyle, TrailingComma,
};

/// Destack format options.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DestackFormatOptions {
    // source
    /// The source language type.
    pub language_type: LanguageType = LanguageType::Destack,

    // layout
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u16 = 100,

    // punctuation and quoting
    /// Quote style for string literals.
    pub quote_style: QuoteStyle = QuoteStyle::Semantic,
    /// Trailing comma rules for multi-line constructs.
    pub trailing_comma: TrailingComma = TrailingComma::All,
    /// Spaces inside object braces: `{ foo }` (true) vs `{foo}` (false).
    pub bracket_spacing: bool = true,
    /// Arrow function parentheses rules.
    pub arrow_parentheses: ArrowParentheses = ArrowParentheses::Always,
    /// Object property quoting rules.
    pub quote_props: QuoteProperty = QuoteProperty::AsNeeded,

    // tree forms
    /// Put `>` of multi-line tree/JSX on same line as last attribute.
    pub bracket_same_line: bool = false,
    /// Force each tree/JSX attribute onto its own line.
    pub single_attribute_per_line: bool = false,

    // imports
    /// Whether to organize/sort imports and exports.
    pub organize_imports: OrganizeImports = OrganizeImports::Off,
    /// Sort order for import/export specifiers within `{ }`.
    pub import_sort_order: ImportSortOrder = ImportSortOrder::Natural,
    /// Respect file-level formatter ignore directives.
    pub respect_file_ignore: bool = true,
}

impl DestackFormatOptions {
    /// Default options with a given line width.
    pub fn default_with_line_width(line_width: u16) -> Self {
        Self {
            line_width,
            ..Self::default()
        }
    }

    /// Default options with tab indent style.
    pub fn default_tab() -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Default options with tab indent style and a given line width.
    pub fn default_tab_with_line_width(line_width: u16) -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            line_width,
            ..Self::default()
        }
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

    /// Convert to print options.
    pub fn print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width.min(255) as u8,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: true,
        }
    }

    /// Create from workspace FormatterOptions.
    pub fn from_formatter_options(options: FormatterOptions, language_type: LanguageType) -> Self {
        Self {
            language_type,
            line_ending: options.line_ending,
            indent_style: options.indent_style,
            indent_width: options.indent_width,
            line_width: options.line_width,
            quote_style: options.quote_style,
            trailing_comma: options.trailing_comma,
            bracket_spacing: options.bracket_spacing,
            arrow_parentheses: options.arrow_parentheses,
            quote_props: options.quote_property,
            bracket_same_line: options.bracket_same_line,
            single_attribute_per_line: options.single_attribute_per_line,
            organize_imports: options.organize_imports,
            import_sort_order: options.import_sort_order,
            respect_file_ignore: true,
        }
    }
}

impl From<FormatterOptions> for DestackFormatOptions {
    fn from(options: FormatterOptions) -> Self {
        Self::from_formatter_options(options, LanguageType::Destack)
    }
}

impl FormatOptions for DestackFormatOptions {
    #[inline]
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    #[inline]
    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    #[inline]
    fn line_width(&self) -> u8 {
        self.line_width.min(255) as u8
    }

    #[inline]
    fn as_print_options(&self) -> PrintOptions {
        self.print_options()
    }
}
