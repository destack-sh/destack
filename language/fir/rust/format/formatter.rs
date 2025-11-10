use std::fmt::Debug;

use crate::format::builder::{FillBuilder, JoinBuilder};
use crate::format::{
    Arguments, Buffer, BufferSnapshot, FormatContext, FormatState, GroupId, PrintResult, VecBuffer,
};
use crate::prelude::*;
use crate::print::{Printed, Printer};

/// Formatting interface for types that can create a formatted representation. The `dyst_fir` equivalent
/// to [`std::fmt::Display`].
pub trait Format<Context> {
    /// Formats the object using the given formatter.
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()>;
}

impl<T, Context> Format<Context> for &T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        Format::format(&**self, f)
    }
}

impl<T, Context> Format<Context> for &mut T
where
    T: ?Sized + Format<Context>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        Format::format(&**self, f)
    }
}

impl<T, Context> Format<Context> for Option<T>
where
    T: Format<Context>,
{
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        match self {
            Some(value) => value.format(f),
            None => Ok(()),
        }
    }
}

impl<Context> Format<Context> for () {
    #[inline]
    fn format(&self, _: &mut Formatter<'_, Context>) -> FormatResult<()> {
        // nothing to do
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
        let source = self.context.file();
        let print_options = self.context.options().as_print_options();

        Printer::new(source, print_options)
    }
}

/// The `write` function takes a target buffer and an `Arguments` struct that can be precompiled with the `format_args!` macro.
/// The arguments will be formatted in-order into the output buffer provided.
#[inline]
pub fn write<Context>(
    output: &mut dyn Buffer<Context = Context>,
    args: Arguments<'_, Context>,
) -> FormatResult<()> {
    let mut f = Formatter::new(output);

    f.write_format(args)
}

/// The `format` function takes an [`Arguments`] struct and returns the resulting formatting IR.
/// The [`Arguments`] instance can be created with the [`format_args!`].
pub fn format<Context>(
    context: Context,
    arguments: Arguments<'_, Context>,
) -> FormatResult<Formatted<Context>>
where
    Context: FormatContext,
{
    let source_length = context.file().len;
    let estimated_buffer_size = source_length / 2;
    let mut state = FormatState::new(context);
    let mut buffer = VecBuffer::with_capacity(estimated_buffer_size as usize, &mut state);

    buffer.write_format(arguments)?;

    let mut document = Document::from(buffer.into_vec());
    document.propagate_expand();

    Ok(Formatted::new(document, state.into_context()))
}

/// Handles the formatting of a AST and stores the context how the AST should be formatted (user preferences).
/// The formatter is passed to the [Format] implementation of every node.so that they
/// can use it to format their children.
pub struct Formatter<'buf, Context> {
    pub(super) buffer: &'buf mut dyn Buffer<Context = Context>,
}

impl<Context> Debug for Formatter<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Formatter").finish()
    }
}

impl<'buf, Context> Formatter<'buf, Context> {
    /// Create a new context that uses the given formatter context.
    pub fn new(buffer: &'buf mut (dyn Buffer<Context = Context> + 'buf)) -> Self {
        Self { buffer }
    }

    /// Get the format options.
    pub fn options(&self) -> &Context::Options
    where
        Context: FormatContext,
    {
        self.context().options()
    }

    /// Get the Context specifying how to format the current AST.
    #[inline]
    pub fn context(&self) -> &Context {
        self.state().context()
    }

    /// Get a mutable reference to the context.
    #[inline]
    pub fn context_mut(&mut self) -> &mut Context {
        self.state_mut().context_mut()
    }

    /// Create a new group id that is unique to this document.
    /// The passed debug name is used in the [`std::fmt::Debug`] of the document if this is a debug build.
    /// The name is unused for production builds and has no meaning on the equality of two group ids.
    #[inline]
    pub fn group_id(&self, debug_name: &'static str) -> GroupId {
        self.state().group_id(debug_name)
    }

    /// Join multiple [Format] together without any separator.
    pub fn join<'a>(&'a mut self) -> JoinBuilder<'a, 'buf, (), Context> {
        JoinBuilder::new(self)
    }

    /// Join the objects by placing the specified separator between every two items.
    pub fn join_with<'a, Joiner>(
        &'a mut self,
        joiner: Joiner,
    ) -> JoinBuilder<'a, 'buf, Joiner, Context>
    where
        Joiner: Format<Context>,
    {
        JoinBuilder::with_separator(self, joiner)
    }

    /// Concatenate a list of [`crate::Format`] objects with spaces and line breaks to fit them on as few lines as possible.
    /// Each node introduces a conceptual group.
    /// The printer first tries to print the item in flat mode but then prints it in expanded mode if it doesn't fit.
    pub fn fill<'a>(&'a mut self) -> FillBuilder<'a, 'buf, Context> {
        FillBuilder::new(self)
    }

    /// Format `content` into an interned node without writing it to the formatter's buffer.
    pub fn intern(&mut self, content: &dyn Format<Context>) -> FormatResult<Option<FormatNode>> {
        let mut buffer = VecBuffer::new(self.state_mut());
        crate::write!(&mut buffer, [content])?;
        let nodes = buffer.into_vec();

        Ok(self.intern_vec(nodes))
    }

    /// Intern a vector of nodes into a single node.
    pub fn intern_vec(&mut self, mut nodes: Vec<FormatNode>) -> Option<FormatNode> {
        match nodes.len() {
            0 => None,
            // doesn't get cheaper than calling clone, use the node directly
            1 => Some(nodes.pop().unwrap()),
            _ => Some(FormatNode::Interned(Interned::new(nodes))),
        }
    }
}

impl<Context> Formatter<'_, Context>
where
    Context: FormatContext,
{
    /// Take a snapshot of the state of the formatter.
    #[inline]
    pub fn state_snapshot(&self) -> FormatterSnapshot {
        FormatterSnapshot {
            buffer: self.buffer.snapshot(),
        }
    }

    /// Restore the state of the formatter to a previous snapshot.
    #[inline]
    pub fn restore_state_snapshot(&mut self, snapshot: FormatterSnapshot) {
        self.buffer.restore_snapshot(snapshot.buffer);
    }
}

impl<Context> Buffer for Formatter<'_, Context> {
    type Context = Context;

    #[inline]
    fn write_node(&mut self, node: FormatNode) {
        self.buffer.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode] {
        self.buffer.nodes()
    }

    #[inline]
    fn write_format(&mut self, arguments: Arguments<'_, Self::Context>) -> FormatResult<()> {
        for argument in arguments.items() {
            argument.format(self)?;
        }
        Ok(())
    }

    fn state(&self) -> &FormatState<Self::Context> {
        self.buffer.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<Self::Context> {
        self.buffer.state_mut()
    }

    fn snapshot(&self) -> BufferSnapshot {
        self.buffer.snapshot()
    }

    fn restore_snapshot(&mut self, snapshot: BufferSnapshot) {
        self.buffer.restore_snapshot(snapshot);
    }
}

/// Snapshot of the formatter state used to handle backtracking if errors are encountered in the formatting process and the formatter has to fallback to printing raw tokens.
/// In practice this only saves the set of printed tokens in debug mode and compiled to nothing in release mode.
#[derive(Debug)]
pub struct FormatterSnapshot {
    buffer: BufferSnapshot,
}

#[cfg(test)]
mod tests {
    use dyst_source::FileType;

    use crate::format::{
        FormatState, Formatted, IndentStyle, SimpleFormatContext, SimpleFormatOptions, VecBuffer,
    };
    use crate::prelude::*;
    use crate::{format, format_args, write};

    /// Join multiple [Format] together without any separator.
    #[test]
    fn test_join_multiple_format_together_without_any_separator() {
        let formatted = format!(
            SimpleFormatContext::empty_dyst(),
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
        let formatted = format!(
            SimpleFormatContext::empty_dyst(),
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
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..Default::default()
                },
                File::empty_with_type(FileType::Dyst)
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
        let entries = [
            token("<b>Important: </b>"),
            token(
                "Please do not commit memory bugs such as segfaults, buffer overflows, etc. otherwise you ",
            ),
            token("<em>will</em>"),
            token(" be reprimanded"),
        ];

        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..Default::default()
                },
                File::empty_with_type(FileType::Dyst)
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
        struct Paragraph(String);

        impl Format<SimpleFormatContext> for Paragraph {
            fn format(&self, f: &mut Formatter<'_, SimpleFormatContext>) -> FormatResult<()> {
                write!(f, [text(&self.0), hard_line_break(),])
            }
        }

        let paragraph = Paragraph(String::from("test"));
        let formatted = format!(SimpleFormatContext::empty_dyst(), [paragraph]).unwrap();

        assert_eq!("test\n", formatted.print().unwrap().as_str());
    }

    /// Write function formats arguments into buffer
    #[test]
    fn test_write_function_formats_arguments_into_buffer() {
        let mut state = FormatState::new(SimpleFormatContext::empty_dyst());
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [format_args!(token("Hello World"))]).unwrap();

        let formatted = Formatted::new(
            Document::from(buffer.into_vec()),
            SimpleFormatContext::empty_dyst(),
        );

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Write macro is preferable for simple cases
    #[test]
    fn test_write_macro_is_preferable_for_simple_cases() {
        let mut state = FormatState::new(SimpleFormatContext::empty_dyst());
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [token("Hello World")]).unwrap();

        let formatted = Formatted::new(
            Document::from(buffer.into_vec()),
            SimpleFormatContext::empty_dyst(),
        );

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Format function creates formatted representation from arguments
    #[test]
    fn test_format_function_creates_formatted_representation_from_arguments() {
        let formatted = format!(
            SimpleFormatContext::empty_dyst(),
            [&format_args!(token("test"))]
        )
        .unwrap();
        assert_eq!("test", formatted.print().unwrap().as_str());
    }

    /// Format macro is preferable for direct usage
    #[test]
    fn test_format_macro_is_preferable_for_direct_usage() {
        let formatted = format!(SimpleFormatContext::empty_dyst(), [token("test")]).unwrap();
        assert_eq!("test", formatted.print().unwrap().as_str());
    }
}
