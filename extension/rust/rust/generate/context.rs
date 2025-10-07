use dyst_fir::format::{FormatContext, FormatOptions, Formatter, IndentStyle, LineEnding};
use dyst_fir::print::PrintOptions;
use dyst_session::Session;
use dyst_source::{Source, Span};
use dyst_token::TokenSpan;

pub type RustFormatter<'ast, 'buf> = Formatter<'buf, RustFormatContext<'ast>>;

/// Rust format options (mostly for testing).
#[derive(Debug, Default, PartialEq, Clone)]
pub struct RustFormatOptions {
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl RustFormatOptions {
    /// Default options with a given line width.
    pub fn default_with_line_width(line_width: u8) -> Self {
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
    pub fn default_tab_with_line_width(line_width: u8) -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            line_width,
            ..Self::default()
        }
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

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Convert to print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}

impl FormatOptions for RustFormatOptions {
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
        self.as_print_options()
    }
}

/// Dyst format context.
#[derive(Debug, Clone)]
pub struct RustFormatContext<'ast> {
    /// The format options.
    pub options: RustFormatOptions,
    /// The source.
    pub source: &'ast Source,
    /// The session.
    pub session: &'ast Session,
}

impl<'ast> RustFormatContext<'ast> {
    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'ast str {
        &self.source.content[span.start as usize..span.end as usize]
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'ast str {
        &self.source.content[token.span.start as usize..token.span.end as usize]
    }
}

impl FormatContext for RustFormatContext<'_> {
    type Options = RustFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn source(&self) -> &Source {
        self.source
    }
}
