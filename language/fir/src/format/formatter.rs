use std::fmt::Debug;

use crate::format::builder::{FillBuilder, JoinBuilder};
use crate::format::{
    Allocator, Arguments, Buffer, FormatContext, FormatState, GroupId, PrintResult, VecBuffer,
};
use crate::prelude::*;
use crate::print::{PrintOptions, Printed, Printer};

/// Formatting interface equivalent to [`std::fmt::Display`] for FIR nodes.
pub trait Format<'a, Context> {
    /// Format this value into FIR nodes.
    fn format(&self, f: &mut Formatter<'_, 'a, Context>) -> FormatResult<()>;
}

impl<'a, T, Context> Format<'a, Context> for &T
where
    T: ?Sized + Format<'a, Context>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        Format::format(&**self, f)
    }
}

impl<'a, T, Context> Format<'a, Context> for &mut T
where
    T: ?Sized + Format<'a, Context>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        Format::format(&**self, f)
    }
}

impl<'a, T, Context> Format<'a, Context> for Option<T>
where
    T: Format<'a, Context>,
{
    fn format(&self, f: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        match self {
            Some(value) => value.format(f),
            None => Ok(()),
        }
    }
}

impl<'a, Context> Format<'a, Context> for () {
    #[inline]
    fn format(&self, _: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        // nothing to do
        Ok(())
    }
}

/// One completed FIR document and its formatting context.
#[derive(Debug, Clone, PartialEq)]
pub struct Formatted<'a, Context> {
    document: Document<'a>,
    context: Context,
}

impl<'a, Context> Formatted<'a, Context> {
    /// Create one completed formatted document.
    pub fn new(document: Document<'a>, context: Context) -> Self {
        Self { document, context }
    }

    /// Return the context used during formatting.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Return the formatted document.
    pub fn document(&self) -> &Document<'a> {
        &self.document
    }

    /// Return the formatted document and discard its context.
    pub fn into_document(self) -> Document<'a> {
        self.document
    }
}

impl<Context> Formatted<'_, Context>
where
    Context: FormatContext,
{
    /// Print this document with its context options.
    pub fn print(&self) -> PrintResult<Printed> {
        let printer = self.create_printer();
        printer.print(&self.document)
    }

    /// Print this document with explicit print options.
    pub fn print_with_options(&self, print_options: PrintOptions) -> PrintResult<Printed> {
        let source = self.context.file();
        let printer = Printer::new(source, print_options);

        printer.print(&self.document)
    }

    /// Print this document beginning at one indentation level.
    pub fn print_with_indent(&self, indent: u16) -> PrintResult<Printed> {
        let printer = self.create_printer();
        printer.print_with_indent(&self.document, indent)
    }

    /// Create a printer from this document's context.
    fn create_printer(&self) -> Printer<'_> {
        let source = self.context.file();
        let print_options = self.context.options().as_print_options();

        Printer::new(source, print_options)
    }
}

/// Write preconstructed arguments into one FIR buffer.
#[inline]
pub fn write<'a, Context>(
    output: &mut dyn Buffer<'a, Context = Context>,
    args: Arguments<'_, 'a, Context>,
) -> FormatResult<()> {
    let mut f = Formatter::new(output);

    f.write_format(args)
}

/// Format preconstructed arguments into one arena-backed document.
pub fn format<'a, Context>(
    allocator: &'a Allocator,
    context: Context,
    arguments: Arguments<'_, 'a, Context>,
) -> FormatResult<Formatted<'a, Context>>
where
    Context: FormatContext,
{
    let mut state = FormatState::new(context, allocator);
    let mut buffer = VecBuffer::new(&mut state);

    buffer.write_format(arguments)?;

    let mut document = Document::from(buffer.into_vec());
    document.propagate_expand();

    Ok(Formatted::new(document, state.into_context()))
}

/// One FIR writer over a formatting buffer and its shared state.
pub struct Formatter<'buf, 'a, Context> {
    pub(super) buffer: &'buf mut dyn Buffer<'a, Context = Context>,
}

impl<Context> Debug for Formatter<'_, '_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Formatter").finish()
    }
}

impl<'buf, 'a, Context> Formatter<'buf, 'a, Context> {
    /// Create a formatter over one buffer.
    pub fn new(buffer: &'buf mut (dyn Buffer<'a, Context = Context> + 'buf)) -> Self {
        Self { buffer }
    }

    /// Return the formatter arena.
    pub fn allocator(&self) -> &'a Allocator {
        self.state().allocator()
    }

    /// Return the format options.
    pub fn options(&self) -> &Context::Options
    where
        Context: FormatContext,
    {
        self.context().options()
    }

    /// Return the formatting context.
    #[inline]
    pub fn context(&self) -> &Context {
        self.state().context()
    }

    /// Return the formatting context mutably.
    #[inline]
    pub fn context_mut(&mut self) -> &mut Context {
        self.state_mut().context_mut()
    }

    /// Create a group ID unique within this document.
    /// The passed debug name is used in the [`std::fmt::Debug`] of the document if this is a debug build.
    /// The name is unused for production builds and has no meaning on the equality of two group ids.
    #[inline]
    pub fn group_id(&mut self, debug_name: &'static str) -> GroupId {
        self.state_mut().group_id(debug_name)
    }

    /// Join formatted entries without a separator.
    pub fn join<'fmt>(&'fmt mut self) -> JoinBuilder<'fmt, 'buf, 'a, (), Context> {
        JoinBuilder::new(self)
    }

    /// Join formatted entries with a separator.
    pub fn join_with<'fmt, Joiner>(
        &'fmt mut self,
        joiner: Joiner,
    ) -> JoinBuilder<'fmt, 'buf, 'a, Joiner, Context>
    where
        Joiner: Format<'a, Context>,
    {
        JoinBuilder::with_separator(self, joiner)
    }

    /// Fill lines with formatted entries and separators.
    pub fn fill<'fmt>(&'fmt mut self) -> FillBuilder<'fmt, 'buf, 'a, Context> {
        FillBuilder::new(self)
    }

    /// Capture `content` as one optional node without writing it to this formatter's buffer.
    pub fn capture(
        &mut self,
        content: &dyn Format<'a, Context>,
    ) -> FormatResult<Option<FormatNode<'a>>> {
        let mut buffer = VecBuffer::new(self.state_mut());
        crate::write!(&mut buffer, [content])?;
        let nodes = buffer.into_vec();

        Ok(nodes.collapse())
    }
}

impl<'a, Context> Buffer<'a> for Formatter<'_, 'a, Context> {
    type Context = Context;

    #[inline]
    fn write_node(&mut self, node: FormatNode<'a>) {
        self.buffer.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode<'a>] {
        self.buffer.nodes()
    }

    #[inline]
    fn write_format(&mut self, arguments: Arguments<'_, 'a, Self::Context>) -> FormatResult<()> {
        for argument in arguments.items() {
            argument.format(self)?;
        }
        Ok(())
    }

    fn state(&self) -> &FormatState<'a, Self::Context> {
        self.buffer.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context> {
        self.buffer.state_mut()
    }
}

#[cfg(test)]
mod tests {
    use destack_source::FileType;

    use crate::format::{
        FormatState, Formatted, IndentStyle, SimpleFormatContext, SimpleFormatOptions, VecBuffer,
    };
    use crate::prelude::*;
    use crate::{format, format_args, write};

    /// Join multiple [Format] together without any separator.
    #[test]
    fn test_join_multiple_format_together_without_any_separator() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [format_with(|f| {
                f.join()
                    .entry(&token("a"))
                    .entry(&space())
                    .entry(&token("+"))
                    .entry(&space())
                    .entry(&token("b"))
                    .finish()
            })]
        )
        .unwrap();

        assert_eq!("a + b", formatted.print().unwrap().as_str());
    }

    /// Join the objects by placing the specified separator between every two items.
    #[test]
    fn test_join_with_separator() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [format_with(|f| {
                f.join_with(&format_args!(token(","), space()))
                    .entry(&token("1"))
                    .entry(&token("2"))
                    .entry(&token("3"))
                    .entry(&token("4"))
                    .finish()
            })]
        )
        .unwrap();

        assert_eq!("1, 2, 3, 4", formatted.print().unwrap().as_str());
    }

    /// Concatenate a list of [`crate::Format`] objects with spaces and line breaks to fit them on as few lines as possible.
    #[test]
    fn test_fill_with_line_breaks() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..Default::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [format_with(|f| {
                f.fill()
                    .entry(
                        &soft_line_break_or_space(),
                        &token("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                    )
                    .entry(
                        &soft_line_break_or_space(),
                        &token("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                    )
                    .entry(
                        &soft_line_break_or_space(),
                        &token("cccccccccccccccccccccccccccccc"),
                    )
                    .entry(
                        &soft_line_break_or_space(),
                        &token("dddddddddddddddddddddddddddddd"),
                    )
                    .finish()
            })]
        )
        .unwrap();

        assert_eq!(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\ncccccccccccccccccccccccccccccc dddddddddddddddddddddddddddddd",
            formatted.print().unwrap().as_str()
        );
    }

    /// Fill with entries from iterator
    #[test]
    fn test_fill_with_entries_from_iterator() {
        let allocator = Allocator::default();
        let entries = [
            token("<b>Important: </b>"),
            token(
                "Please do not commit memory bugs such as segfaults, buffer overflows, etc. otherwise you ",
            ),
            token("<em>will</em>"),
            token(" be reprimanded"),
        ];

        let formatted = format!(
            &allocator,
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..Default::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [format_with(|f| {
                f.fill()
                    .entries(&soft_line_break(), entries.iter())
                    .finish()
            })]
        )
        .unwrap();

        assert_eq!(
            &"<b>Important: </b>\nPlease do not commit memory bugs such as segfaults, buffer overflows, etc. otherwise you \n<em>will</em> be reprimanded".to_string(),
            formatted.print().unwrap().as_str()
        );
    }

    /// Format interface creates formatted representation
    #[test]
    fn test_format_interface_creates_formatted_representation() {
        let allocator = Allocator::default();
        struct Paragraph(String);

        impl<'a> Format<'a, SimpleFormatContext> for Paragraph {
            fn format(&self, f: &mut Formatter<'_, 'a, SimpleFormatContext>) -> FormatResult<()> {
                write!(f, [copied_text(&self.0), hard_line_break(),])
            }
        }

        let paragraph = Paragraph(String::from("test"));
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [paragraph]
        )
        .unwrap();

        assert_eq!("test\n", formatted.print().unwrap().as_str());
    }

    /// Write function formats arguments into buffer
    #[test]
    fn test_write_function_formats_arguments_into_buffer() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [format_args!(token("Hello World"))]).unwrap();

        let formatted = Formatted::new(
            Document::from(buffer.into_vec()),
            SimpleFormatContext::empty_destack(),
        );

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Write macro is preferable for simple cases
    #[test]
    fn test_write_macro_is_preferable_for_simple_cases() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [token("Hello World")]).unwrap();

        let formatted = Formatted::new(
            Document::from(buffer.into_vec()),
            SimpleFormatContext::empty_destack(),
        );

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Format function creates formatted representation from arguments
    #[test]
    fn test_format_function_creates_formatted_representation_from_arguments() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [&format_args!(token("test"))]
        )
        .unwrap();
        assert_eq!("test", formatted.print().unwrap().as_str());
    }

    /// Format macro is preferable for direct usage
    #[test]
    fn test_format_macro_is_preferable_for_direct_usage() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [token("test")]
        )
        .unwrap();
        assert_eq!("test", formatted.print().unwrap().as_str());
    }
}
