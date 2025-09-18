//// Formatting trait for types that can create a formatted representation. The `ruff_formatter` equivalent
/// to [`std::fmt::Display`].
///
/// ## Example
/// Implementing `Format` for a custom struct
///
/// ```
/// use ruff_formatter::{format, write, IndentStyle};
/// use ruff_formatter::prelude::*;
/// use ruff_text_size::TextSize;
///
/// struct Paragraph(String);
///
/// impl Format<SimpleFormatContext> for Paragraph {
///     fn fmt(&self, f: &mut Formatter<SimpleFormatContext>) -> FormatResult<()> {
///         write!(f, [
///             text(&self.0),
///             hard_line_break(),
///         ])
///     }
/// }
///
/// # fn main() -> FormatResult<()> {
/// let paragraph = Paragraph(String::from("test"));
/// let formatted = format!(SimpleFormatContext::default(), [paragraph])?;
///
/// assert_eq!("test\n", formatted.print()?.as_code());
/// # Ok(())
/// # }
/// ```
pub trait Format<Context> {
    /// Formats the object using the given formatter.
    fn fmt(&self, f: &mut Formatter<Context>) -> FormatResult<()>;
}

impl<T, Context> Format<Context> for &T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<Context>) -> FormatResult<()> {
        Format::fmt(&**self, f)
    }
}

impl<T, Context> Format<Context> for &mut T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<Context>) -> FormatResult<()> {
        Format::fmt(&**self, f)
    }
}

impl<T, Context> Format<Context> for Option<T>
where
    T: Format<Context>,
{
    fn fmt(&self, f: &mut Formatter<Context>) -> FormatResult<()> {
        match self {
            Some(value) => value.fmt(f),
            None => Ok(()),
        }
    }
}

impl<Context> Format<Context> for () {
    #[inline]
    fn fmt(&self, _: &mut Formatter<Context>) -> FormatResult<()> {
        // Intentionally left empty
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Formatted<Context> {
    document: Document,
    context: Context,
}

impl<Context> Formatted<Context> {
    pub fn new(document: Document, context: Context) -> Self {
        Self { document, context }
    }

    /// Returns the context used during formatting.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Returns the formatted document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Consumes `self` and returns the formatted document.
    pub fn into_document(self) -> Document {
        self.document
    }
}

impl<Context> Formatted<Context>
where
    Context: FormatContext,
{
    pub fn print(&self) -> PrintResult<Printed> {
        let printer = self.create_printer();
        printer.print(&self.document)
    }

    pub fn print_with_indent(&self, indent: u16) -> PrintResult<Printed> {
        let printer = self.create_printer();
        printer.print_with_indent(&self.document, indent)
    }

    fn create_printer(&self) -> Printer<'_> {
        let source_code = self.context.source_code();
        let print_options = self.context.options().as_print_options();

        Printer::new(source_code, print_options)
    }
}

impl<Context> Display for Formatted<Context>
where
    Context: FormatContext,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.document.display(self.context.source_code()), f)
    }
}
