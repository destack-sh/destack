mod array;
mod object;
mod trivia;
mod value;

use destack_fir::format::{
    Document, FormatContext, FormatOptions, FormatResult, FormatState, Formatter, VecBuffer,
};
use destack_fir::print::{PrintOptions, Printer};
use destack_repository::FormatterOptions;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::JsonDocument;

pub use array::*;
pub use object::*;
pub use trivia::*;
pub use value::*;

/// JSON formatter type alias.
pub type JsonFormatter<'buf> = Formatter<'buf, JsonFormatContext>;

/// JSON format options.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct JsonFormatOptions {
    /// The type of line ending to apply.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style (spaces or tabs).
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Number of spaces per indent level.
    pub indent_width: u8 = 2,
    /// Maximum line length (best effort).
    pub line_width: u8 = 80,
    /// Whether to add trailing commas in arrays and objects.
    pub trailing_comma: bool = false,
}

impl JsonFormatOptions {
    /// Create options with space indentation.
    pub fn with_spaces(indent_width: u8) -> Self {
        Self {
            indent_style: IndentStyle::Space,
            indent_width,
            ..Self::default()
        }
    }

    /// Create options with tab indentation.
    pub fn with_tabs() -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            indent_width: 1,
            ..Self::default()
        }
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Set trailing comma behavior.
    pub fn with_trailing_comma(mut self, trailing_comma: bool) -> Self {
        self.trailing_comma = trailing_comma;
        self
    }

    /// Convert to print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: true,
        }
    }
}

impl FormatOptions for JsonFormatOptions {
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

/// Convert workspace formatter options to JSON format options.
impl From<FormatterOptions> for JsonFormatOptions {
    fn from(opts: FormatterOptions) -> Self {
        Self {
            line_ending: opts.line_ending,
            indent_style: opts.indent_style,
            indent_width: opts.indent_width,
            line_width: opts.line_width as u8,
            trailing_comma: false,
        }
    }
}

/// JSON format context.
#[derive(Debug)]
pub struct JsonFormatContext {
    /// The format options.
    pub options: JsonFormatOptions,
    /// The file being formatted.
    pub file: File,
}

impl JsonFormatContext {
    /// Create a new format context with the given options.
    pub fn new(options: JsonFormatOptions) -> Self {
        Self {
            options,
            file: File::empty_text(FileType::Json),
        }
    }

    /// Create a format context with default options.
    pub fn default_options() -> Self {
        Self::new(JsonFormatOptions::default())
    }

    /// Whether to add trailing commas.
    pub fn trailing_comma(&self) -> bool {
        self.options.trailing_comma
    }
}

impl FormatContext for JsonFormatContext {
    type Options = JsonFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Format a JSON document to a string.
pub fn format_json(document: &JsonDocument, options: &JsonFormatOptions) -> String {
    // create the format context
    let context = JsonFormatContext::new(*options);
    let mut state = FormatState::new(context);
    let mut buffer = VecBuffer::new(&mut state);

    // format the document
    {
        let mut formatter = Formatter::new(&mut buffer);
        let _ = format_document(document, &mut formatter);
    }

    // build the document
    let fir_document = Document::from(buffer.into_vec());

    // print to string
    let file = File::empty_text(FileType::Json);
    let print_options = options.as_print_options();

    match Printer::new(&file, print_options).print(&fir_document) {
        Ok(printed) => printed.into_str(),
        Err(_) => String::new(),
    }
}

/// Format a JSON document.
fn format_document(document: &JsonDocument, f: &mut JsonFormatter<'_>) -> FormatResult<()> {
    // format leading trivia
    format_trivia_list(&document.trivia_before, f)?;

    // format the root value
    format_value(&document.value, f)?;

    // format trailing trivia
    format_trivia_list(&document.trivia_after, f)?;

    Ok(())
}
