use destack_fir::format::{FormatContext, FormatOptions};
use destack_fir::print::PrintOptions;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::{Doctype, DoctypeQuoteStyle, Element, SelfClosingStyle, StringId, Tree};

/// HTML format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HtmlFormatOptions {
    /// The line ending to apply to formatted output.
    pub line_ending: LineEnding,
    /// The indent style.
    pub indent_style: IndentStyle,
    /// The indent width.
    pub indent_width: u8,
    /// The target line width.
    pub line_width: u8,
}

impl Default for HtmlFormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl HtmlFormatOptions {
    /// Return pretty HTML format options.
    pub fn pretty() -> Self {
        Self::default()
    }

    /// Set the line ending.
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

    /// Set the target line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }
}

impl FormatOptions for HtmlFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: true,
        }
    }
}

/// One HTML FIR formatting context.
#[derive(Debug, Clone)]
pub(crate) struct HtmlFormatContext {
    /// The format options.
    options: HtmlFormatOptions,
    /// The virtual HTML file used by FIR printing.
    file: File,
}

impl HtmlFormatContext {
    /// Create one HTML formatting context.
    pub(crate) fn new(options: HtmlFormatOptions) -> Self {
        Self {
            options,
            file: File::empty_text(FileType::Html),
        }
    }

    /// Return one authored element start tag name.
    pub(crate) fn element_start_tag_name(&self, element: &Element) -> Option<StringId> {
        element.authored_start_tag_name
    }

    /// Return one authored element end tag name.
    pub(crate) fn element_end_tag_name(&self, element: &Element) -> Option<StringId> {
        element
            .authored_end_tag_name
            .or(element.authored_start_tag_name)
    }

    /// Render one authored doctype keyword.
    pub(crate) fn render_doctype_keyword(&self, tree: &Tree, doctype: &Doctype) -> String {
        tree.string(doctype.doctype_keyword).to_string()
    }

    /// Render one authored doctype kind keyword.
    pub(crate) fn render_doctype_kind_keyword(
        &self,
        tree: &Tree,
        doctype: &Doctype,
        fallback: &str,
    ) -> String {
        doctype
            .kind_keyword
            .map(|keyword| tree.string(keyword).to_string())
            .unwrap_or_else(|| fallback.to_string())
    }

    /// Render one authored doctype quote delimiter.
    pub(crate) fn render_doctype_quote(quote_style: DoctypeQuoteStyle) -> &'static str {
        match quote_style {
            DoctypeQuoteStyle::DoubleQuoted => "\"",
            DoctypeQuoteStyle::SingleQuoted => "'",
        }
    }

    /// Render one authored self closing delimiter.
    pub(crate) fn render_self_closing_delimiter(&self, element: &Element) -> &'static str {
        match element
            .self_closing_style
            .unwrap_or(SelfClosingStyle::Spaced)
        {
            SelfClosingStyle::Compact => "/>",
            SelfClosingStyle::Spaced => " />",
        }
    }
}

impl FormatContext for HtmlFormatContext {
    type Options = HtmlFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}
