use std::fmt::Debug;

use crate::format::builder::{FillBuilder, JoinBuilder};
use crate::format::{
    Allocator, Arguments, BestFittingMode, Condition, DedentMode, FormatContext, FormatElement,
    FormatState, FormatTag, GroupId, InstructionTape, LineMode, Opcode, PrintMode, PrintResult,
    VerbatimKind, encode_group_id, encode_text_layout, encode_width,
};
use crate::prelude::*;
use crate::print::{PrintOptions, Printed, Printer};

/// Formatting interface equivalent to [`std::fmt::Display`] for FIR operations.
pub trait Format<'a, Context> {
    /// Format this value into FIR operations.
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
#[derive(Debug)]
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

/// Write preconstructed arguments into one formatter.
#[inline]
pub fn write<'a, Context>(
    output: &mut Formatter<'_, 'a, Context>,
    args: Arguments<'_, 'a, Context>,
) -> FormatResult<()> {
    output.write_format(args)
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
    let mut formatter = Formatter::new(&mut state);

    formatter.write_format(arguments)?;

    let instructions = formatter.into_tape().into_slice();
    let (context, groups, fits_expanded) = state.finish();
    let document = Document::new(instructions, groups, fits_expanded);

    Ok(Formatted::new(document, context))
}

/// One FIR writer over shared formatting state and arena storage.
pub struct Formatter<'state, 'a, Context> {
    /// The shared formatting state.
    state: &'state mut FormatState<'a, Context>,
    /// The FIR instructions written by this formatter.
    instructions: InstructionTape<'a>,
}

impl<Context> Debug for Formatter<'_, '_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Formatter").finish()
    }
}

impl<'state, 'a, Context> Formatter<'state, 'a, Context> {
    /// Create an empty formatter over shared state.
    pub fn new(state: &'state mut FormatState<'a, Context>) -> Self {
        let instructions = InstructionTape::new(state.allocator());

        Self {
            state,
            instructions,
        }
    }

    /// Create an empty formatter with instruction capacity.
    pub fn with_capacity(capacity: usize, state: &'state mut FormatState<'a, Context>) -> Self {
        let instructions = InstructionTape::with_capacity(capacity, state.allocator());

        Self {
            state,
            instructions,
        }
    }

    /// Return the written FIR instruction tape.
    pub fn into_tape(self) -> InstructionTape<'a> {
        self.instructions
    }

    /// Encode one formatting operation.
    #[inline(always)]
    pub fn write_element(&mut self, element: FormatElement<'a>) {
        match element {
            FormatElement::Space => self.write_opcode(Opcode::Space, 0),
            FormatElement::Line(LineMode::Soft) => self.write_opcode(Opcode::SoftLine, 0),
            FormatElement::Line(LineMode::SoftOrSpace) => {
                self.write_opcode(Opcode::SoftOrSpaceLine, 0);
            }
            FormatElement::Line(LineMode::Hard) => self.write_opcode(Opcode::HardLine, 0),
            FormatElement::Line(LineMode::Empty) => self.write_opcode(Opcode::EmptyLine, 0),
            FormatElement::ExpandParent => self.write_opcode(Opcode::ExpandParent, 0),
            FormatElement::Token { text } => {
                self.write_operand(
                    Opcode::Token,
                    text.len() as u64,
                    text.as_ptr() as usize as u64,
                );
            }
            FormatElement::Text { text, width } => {
                let layout = encode_text_layout(text, width);
                self.write_operand(Opcode::Text, layout, text.as_ptr() as usize as u64);
            }
            FormatElement::SourcePosition { source } => {
                self.write_opcode(Opcode::SourcePosition, u64::from(source));
            }
            FormatElement::FileSlice { range, width } => {
                let offsets = u64::from(range.start) | (u64::from(range.end) << 32);
                self.write_operand(Opcode::FileSlice, encode_width(width), offsets);
            }
            FormatElement::LineSuffixBoundary => {
                self.write_opcode(Opcode::LineSuffixBoundary, 0);
            }
            FormatElement::Slice(slice) => {
                self.write_operand(Opcode::Slice, 0, slice.as_ptr() as usize as u64);
            }
            FormatElement::BestFitting { variants, mode } => {
                let opcode = match mode {
                    BestFittingMode::FirstLine => Opcode::BestFittingFirstLine,
                    BestFittingMode::AllLines => Opcode::BestFittingAllLines,
                };
                let slices = variants.as_slice();

                self.write_operand(opcode, slices.len() as u64, slices.as_ptr() as usize as u64);
            }
            FormatElement::Tag(tag) => self.write_tag(tag),
        }
    }

    /// Encode one structural formatting tag.
    fn write_tag(&mut self, tag: FormatTag) {
        use FormatTag::*;

        match tag {
            StartIndent => self.write_opcode(Opcode::StartIndent, 0),
            EndIndent => self.write_opcode(Opcode::EndIndent, 0),
            StartAlign(width) => {
                self.write_opcode(Opcode::StartAlign, u64::from(width));
            }
            EndAlign => self.write_opcode(Opcode::EndAlign, 0),
            StartDedent(DedentMode::Level) => {
                self.write_opcode(Opcode::StartDedentLevel, 0);
            }
            EndDedent(DedentMode::Level) => {
                self.write_opcode(Opcode::EndDedentLevel, 0);
            }
            StartDedent(DedentMode::Root) => {
                self.write_opcode(Opcode::StartDedentRoot, 0);
            }
            EndDedent(DedentMode::Root) => {
                self.write_opcode(Opcode::EndDedentRoot, 0);
            }
            StartGroup(group) => {
                let index = self.state.push_group(group);
                let payload = u64::from(index.value()) | ((group.mode() as u64) << 32);
                self.write_opcode(Opcode::StartGroup, payload);
            }
            EndGroup => self.write_opcode(Opcode::EndGroup, 0),
            StartConditionalGroup(group) => {
                let index = self.state.push_conditional_group(group);
                self.write_opcode(Opcode::StartConditionalGroup, u64::from(index.value()));
            }
            EndConditionalGroup => {
                self.write_opcode(Opcode::EndConditionalGroup, 0);
            }
            StartConditionalContent(Condition { mode, group_id }) => {
                let opcode = match mode {
                    PrintMode::Flat => Opcode::StartConditionalFlat,
                    PrintMode::Expanded => Opcode::StartConditionalExpanded,
                };
                self.write_opcode(opcode, u64::from(encode_group_id(group_id)));
            }
            EndConditionalContent => {
                self.write_opcode(Opcode::EndConditionalContent, 0);
            }
            StartIndentIfGroupBreaks(group_id) => {
                self.write_opcode(
                    Opcode::StartIndentIfGroupBreaks,
                    u64::from(u32::from(group_id)),
                );
            }
            EndIndentIfGroupBreaks(group_id) => {
                self.write_opcode(
                    Opcode::EndIndentIfGroupBreaks,
                    u64::from(u32::from(group_id)),
                );
            }
            StartFill => self.write_opcode(Opcode::StartFill, 0),
            EndFill => self.write_opcode(Opcode::EndFill, 0),
            StartEntry => self.write_opcode(Opcode::StartEntry, 0),
            EndEntry => self.write_opcode(Opcode::EndEntry, 0),
            StartLineSuffix => self.write_opcode(Opcode::StartLineSuffix, 0),
            EndLineSuffix => self.write_opcode(Opcode::EndLineSuffix, 0),
            StartVerbatim(VerbatimKind::Bogus) => {
                self.write_opcode(Opcode::StartVerbatimBogus, 0);
            }
            StartVerbatim(VerbatimKind::Suppressed) => {
                self.write_opcode(Opcode::StartVerbatimSuppressed, 0);
            }
            StartVerbatim(VerbatimKind::Verbatim { length }) => {
                self.write_opcode(Opcode::StartVerbatim, u64::from(length));
            }
            EndVerbatim => self.write_opcode(Opcode::EndVerbatim, 0),
            StartFitsExpanded(fits) => {
                let index = self.state.push_fits_expanded(fits);
                self.write_opcode(Opcode::StartFitsExpanded, u64::from(index.value()));
            }
            EndFitsExpanded => self.write_opcode(Opcode::EndFitsExpanded, 0),
            StartBestFitParenthesize { id } => {
                self.write_opcode(
                    Opcode::StartBestFitParenthesize,
                    u64::from(encode_group_id(id)),
                );
            }
            EndBestFitParenthesize => {
                self.write_opcode(Opcode::EndBestFitParenthesize, 0);
            }
        }
    }

    /// Write preconstructed format arguments.
    #[inline]
    pub fn write_format(&mut self, arguments: Arguments<'_, 'a, Context>) -> FormatResult<()> {
        for argument in arguments.items() {
            argument.format(self)?;
        }

        Ok(())
    }

    /// Return the shared formatting state.
    pub fn state(&self) -> &FormatState<'a, Context> {
        self.state
    }

    /// Return the shared formatting state mutably.
    pub fn state_mut(&mut self) -> &mut FormatState<'a, Context> {
        self.state
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
    #[inline]
    pub fn group_id(&mut self) -> GroupId {
        self.state_mut().group_id()
    }

    /// Join formatted entries without a separator.
    pub fn join<'fmt>(&'fmt mut self) -> JoinBuilder<'fmt, 'state, 'a, (), Context> {
        JoinBuilder::new(self)
    }

    /// Join formatted entries with a separator.
    pub fn join_with<'fmt, Joiner>(
        &'fmt mut self,
        joiner: Joiner,
    ) -> JoinBuilder<'fmt, 'state, 'a, Joiner, Context>
    where
        Joiner: Format<'a, Context>,
    {
        JoinBuilder::with_separator(self, joiner)
    }

    /// Fill lines with formatted entries and separators.
    pub fn fill<'fmt>(&'fmt mut self) -> FillBuilder<'fmt, 'state, 'a, Context> {
        FillBuilder::new(self)
    }

    /// Capture content as one optional nested instruction.
    pub fn capture<Content>(&mut self, content: &Content) -> FormatResult<Option<FormatElement<'a>>>
    where
        Content: ?Sized + Format<'a, Context>,
    {
        let instructions = self.capture_tape(content)?;

        Ok(instructions.collapse())
    }

    /// Format content into a separate FIR instruction tape.
    pub(crate) fn capture_tape<Content>(
        &mut self,
        content: &Content,
    ) -> FormatResult<InstructionTape<'a>>
    where
        Content: ?Sized + Format<'a, Context>,
    {
        let mut formatter = Formatter::new(self.state_mut());
        content.format(&mut formatter)?;

        Ok(formatter.into_tape())
    }

    /// Encode one opcode header.
    #[inline(always)]
    fn write_opcode(&mut self, opcode: Opcode, payload: u64) {
        self.instructions.push(opcode.encode(payload));
    }

    /// Encode one opcode with a full operand.
    #[inline(always)]
    fn write_operand(&mut self, opcode: Opcode, payload: u64, operand: u64) {
        self.instructions.push(opcode.encode_with(payload, operand));
    }
}

#[cfg(test)]
mod tests {
    use tspp_source::FileType;

    use crate::format::{
        FormatState, Formatted, IndentStyle, SimpleFormatContext, SimpleFormatOptions,
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
                File::empty_text(FileType::Tspp)
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
                File::empty_text(FileType::Tspp)
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

    /// Write arguments into one formatter.
    #[test]
    fn test_write_arguments() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut formatter = Formatter::new(&mut state);

        write!(&mut formatter, [format_args!(token("Hello World"))]).unwrap();

        let instructions = formatter.into_tape().into_slice();
        let (context, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);
        let formatted = Formatted::new(document, context);

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Write simple format values.
    #[test]
    fn test_write_values() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut formatter = Formatter::new(&mut state);

        write!(&mut formatter, [token("Hello World")]).unwrap();

        let instructions = formatter.into_tape().into_slice();
        let (context, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);
        let formatted = Formatted::new(document, context);

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
