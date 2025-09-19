use std::fmt::Debug;

use crate::{
    Arguments, Buffer, Document, FormatContext, FormatOptions, FormatResult, FormatState,
    Formatter, PrintResult, Printed, Printer, VecBuffer,
};

/// Formatting trait for types that can create a formatted representation. The `dyst_language_format` equivalent
/// to [`std::fmt::Display`].
///
/// ## Example
/// Implementing `Format` for a custom struct
///
/// ```
/// use dyst_language_format::{format, write, IndentStyle};
/// use dyst_language_format::prelude::*;
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
    fn fmt(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()>;
}

impl<T, Context> Format<Context> for &T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        Format::fmt(&**self, f)
    }
}

impl<T, Context> Format<Context> for &mut T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        Format::fmt(&**self, f)
    }
}

impl<T, Context> Format<Context> for Option<T>
where
    T: Format<Context>,
{
    fn fmt(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        match self {
            Some(value) => value.fmt(f),
            None => Ok(()),
        }
    }
}

impl<Context> Format<Context> for () {
    #[inline]
    fn fmt(&self, _: &mut Formatter<'_, Context>) -> FormatResult<()> {
        // Intentionally left empty
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
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
        let source = self.context.source();
        let print_options = self.context.options().as_print_options();

        Printer::new(source, print_options)
    }
}

/// The `write` function takes a target buffer and an `Arguments` struct that can be precompiled with the `format_args!` macro.
///
/// The arguments will be formatted in-order into the output buffer provided.
///
/// # Examples
///
/// ```
/// use dyst_language_format::prelude::*;
/// use dyst_language_format::{VecBuffer, format_args, FormatState, write, Formatted};
///
/// # fn main() -> FormatResult<()> {
/// let mut state = FormatState::new(SimpleFormatContext::default());
/// let mut buffer = VecBuffer::new(&mut state);
///
/// write!(&mut buffer, [format_args!(token("Hello World"))])?;
///
/// let formatted = Formatted::new(Document::from(buffer.into_vec()), SimpleFormatContext::default());
///
/// assert_eq!("Hello World", formatted.print()?.as_code());
/// # Ok(())
/// # }
/// ```
///
/// Please note that using [`write!`] might be preferable. Example:
///
/// ```
/// use dyst_language_format::prelude::*;
/// use dyst_language_format::{VecBuffer, format_args, FormatState, write, Formatted};
///
/// # fn main() -> FormatResult<()> {
/// let mut state = FormatState::new(SimpleFormatContext::default());
/// let mut buffer = VecBuffer::new(&mut state);
///
/// write!(&mut buffer, [token("Hello World")])?;
///
/// let formatted = Formatted::new(Document::from(buffer.into_vec()), SimpleFormatContext::default());
///
/// assert_eq!("Hello World", formatted.print()?.as_code());
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn write<Context>(
    output: &mut dyn Buffer<Context = Context>,
    args: Arguments<'_, Context>,
) -> FormatResult<()> {
    let mut f = Formatter::new(output);

    f.write_fmt(args)
}

/// The `format` function takes an [`Arguments`] struct and returns the resulting formatting IR.
///
/// The [`Arguments`] instance can be created with the [`format_args!`].
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use dyst_language_format::prelude::*;
/// use dyst_language_format::{format, format_args};
///
/// # fn main() -> FormatResult<()> {
/// let formatted = format!(SimpleFormatContext::default(), [&format_args!(token("test"))])?;
/// assert_eq!("test", formatted.print()?.as_code());
/// # Ok(())
/// # }
/// ```
///
/// Please note that using [`format!`] might be preferable. Example:
///
/// ```
/// use dyst_language_format::prelude::*;
/// use dyst_language_format::{format};
///
/// # fn main() -> FormatResult<()> {
/// let formatted = format!(SimpleFormatContext::default(), [token("test")])?;
/// assert_eq!("test", formatted.print()?.as_code());
/// # Ok(())
/// # }
/// ```
pub fn format<Context>(
    context: Context,
    arguments: Arguments<'_, Context>,
) -> FormatResult<Formatted<Context>>
where
    Context: FormatContext,
{
    let source_length = context.source().content.len();
    // Use a simple heuristic to guess the number of expected format elements.
    // See [#6612](https://github.com/astral-sh/ruff/pull/6612) for more details on how the formula was determined. Changes to our formatter, or supporting
    // more languages may require fine tuning the formula.
    let estimated_buffer_size = source_length / 2;
    let mut state = FormatState::new(context);
    let mut buffer = VecBuffer::with_capacity(estimated_buffer_size, &mut state);

    buffer.write_fmt(arguments)?;

    let mut document = Document::from(buffer.into_vec());
    document.propagate_expand();

    Ok(Formatted::new(document, state.into_context()))
}
