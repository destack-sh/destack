use crate::print::{PrintOptions, Printed};

use tspp_source::{File, Span};
use tspp_unicode::UnicodeWidthChar;

use crate::format::{
    ActualStart, BestFittingMode, BestFittingVariants, Condition, DecodedInstruction, DedentMode,
    Document, FileMarker, FormatTagKind, GroupId, GroupMode, IndentStyle, Indentation, Instruction,
    InstructionTag, InvalidDocumentError, LineMode, Opcode, PrintError, PrintMode, PrintResult,
    RequestedOutputBytes, TextWidth, VerbatimKind,
};
use crate::print::call::{
    CallStack, FitsCallStack, FitsIndentStack, IndentStack, PrintArgs, PrintCallStack,
    PrintIndentStack, StackFrame, SuffixStack,
};
use crate::print::line::{LineSuffixEntry, LineSuffixes};
use crate::print::mode::MeasureMode;
use crate::print::queue::{
    AllPredicate, FitsEndPredicate, FitsQueue, PrintQueue, Queue, QueueFrame, SingleEntryPredicate,
};

/// A printer that renders one formatting document into text.
#[derive(Debug)]
pub struct Printer<'a> {
    /// The active printing options.
    options: PrintOptions,
    /// The source file used by verbatim instructions.
    source: &'a File,
    /// The mutable output state.
    state: PrinterState<'a>,
    /// The document installed for one print call.
    document: Option<&'a Document<'a>>,
}

impl<'a> Printer<'a> {
    /// Create one printer for a source file and options.
    pub fn new(source: &'a File, options: PrintOptions) -> Self {
        Self {
            source,
            options,
            state: PrinterState::new(source.len as usize, options.max_output_bytes),
            document: None,
        }
    }

    /// Print one formatting document.
    pub fn print(self, document: &'a Document<'a>) -> PrintResult<Printed> {
        self.print_with_indent(document, 0)
    }

    /// Print one formatting document from the specified indentation level.
    pub fn print_with_indent(
        mut self,
        document: &'a Document<'a>,
        indent: u16,
    ) -> PrintResult<Printed> {
        let indentation = Indentation::Level(indent);
        self.state.pending_indent = indentation;
        self.document = Some(document);

        let mut stack = PrintCallStack::new(PrintArgs::new());
        let mut indent_stack = PrintIndentStack::new(indentation);
        let mut queue = PrintQueue::new(document.instructions());

        loop {
            if let Some(instruction) = queue.pop() {
                self.print_instruction(&mut stack, &mut indent_stack, &mut queue, instruction)?;
            } else if !self.flush_line_suffixes(&mut queue, &mut stack, &mut indent_stack, None) {
                break;
            }
        }

        // push any pending marker
        self.push_marker();

        Ok(Printed::new(
            self.state.buffer,
            None,
            self.state.source_markers,
            self.state.verbatim_markers,
        ))
    }

    /// Print one instruction and queue any nested instructions.
    fn print_instruction(
        &mut self,
        stack: &mut PrintCallStack,
        indent_stack: &mut PrintIndentStack,
        queue: &mut PrintQueue<'a>,
        instruction: Instruction<'a>,
    ) -> PrintResult<()> {
        #[allow(clippy::enum_glob_use)]
        use InstructionTag::*;

        let args = stack.top();

        match instruction.decode() {
            DecodedInstruction::Space => self.print_text(Text::Token(" "))?,
            DecodedInstruction::Token(text) => self.print_text(Text::Token(text))?,
            DecodedInstruction::Text {
                text,
                width: text_width,
            } => self.print_text(Text::Text { text, text_width })?,
            DecodedInstruction::SourcePosition(source) => {
                self.state.pending_source_position = Some(source);
            }
            DecodedInstruction::FileSlice {
                range,
                width: text_width,
            } => {
                self.state.pending_source_position = Some(range.start);
                let text = self.source.get_range_str(range).ok_or_else(|| {
                    PrintError::SourceTextUnavailable {
                        span: Span::new(self.source.id, range.start, range.end),
                    }
                })?;
                self.print_text(Text::Text { text, text_width })?;
                self.state.pending_source_position = Some(range.end);
            }
            DecodedInstruction::Line(line_mode) => {
                if args.mode().is_flat()
                    && matches!(line_mode, LineMode::Soft | LineMode::SoftOrSpace)
                {
                    if line_mode == LineMode::SoftOrSpace {
                        self.print_text(Text::Token(" "))?;
                    }
                } else if self.state.line_suffixes.has_pending() {
                    self.flush_line_suffixes(queue, stack, indent_stack, Some(instruction));
                } else {
                    // only print a newline if the current line isn't already empty
                    if self.state.buffer.len() > self.state.line_start {
                        self.push_marker();
                        self.print_char('\n')?;
                    }

                    // print a second line break if this is an empty line
                    if line_mode == LineMode::Empty {
                        self.push_marker();
                        self.print_char('\n')?;
                    }

                    self.state.pending_indent = indent_stack.indentation();
                }
            }

            DecodedInstruction::ExpandParent => {
                // handled by instruction tape expansion tracking
            }

            DecodedInstruction::LineSuffixBoundary => {
                self.flush_line_suffixes(
                    queue,
                    stack,
                    indent_stack,
                    Some(Instruction::line(LineMode::Hard)),
                );
            }

            DecodedInstruction::BestFitting { variants, mode } => {
                self.print_best_fitting(variants, mode, queue, stack, indent_stack)?;
            }

            DecodedInstruction::Slice(content) => {
                queue.push_slice(content);
            }

            DecodedInstruction::Tag(StartGroup(index, _)) => {
                let group = *self.document().group(index);
                let print_mode = match group.mode() {
                    GroupMode::Expand | GroupMode::Propagated => PrintMode::Expanded,
                    GroupMode::Flat => self.flat_group_print_mode(
                        FormatTagKind::Group,
                        group.id(),
                        args,
                        queue,
                        stack,
                        indent_stack,
                    )?,
                };

                if let Some(id) = group.id() {
                    self.state.group_modes.set_print_mode(id, print_mode);
                }

                stack.push(FormatTagKind::Group, args.with_print_mode(print_mode));
            }

            DecodedInstruction::Tag(StartBestFitParenthesize(id)) => {
                let expanded_start = [
                    Instruction::token("("),
                    Instruction::start_indent(),
                    Instruction::line(LineMode::Hard),
                ];

                let fits_flat = self.flat_group_print_mode(
                    FormatTagKind::BestFitParenthesize,
                    id,
                    args,
                    queue,
                    stack,
                    indent_stack,
                )? == PrintMode::Flat;

                let print_mode = if fits_flat {
                    PrintMode::Flat
                } else {
                    // test if the content fits in expanded mode. If not, prefer avoiding the parentheses
                    // over parenthesizing the expression.
                    if let Some(id) = id {
                        self.state
                            .group_modes
                            .set_print_mode(id, PrintMode::Expanded);
                    }

                    stack.push(
                        FormatTagKind::BestFitParenthesize,
                        args.with_measure_mode(MeasureMode::AllLines),
                    );

                    queue.push_triple(expanded_start);
                    let fits_expanded = self.fits(queue, stack, indent_stack)?;
                    queue.pop_frame();
                    stack.pop(FormatTagKind::BestFitParenthesize)?;

                    if fits_expanded {
                        PrintMode::Expanded
                    } else {
                        PrintMode::Flat
                    }
                };

                if let Some(id) = id {
                    self.state.group_modes.set_print_mode(id, print_mode);
                }

                if print_mode.is_expanded() {
                    // parenthesize the content. The `EndIndent` is handled inside of the `EndBestFitParenthesize`
                    queue.push_triple(expanded_start);
                }

                stack.push(
                    FormatTagKind::BestFitParenthesize,
                    args.with_print_mode(print_mode),
                );
            }

            DecodedInstruction::Tag(EndBestFitParenthesize) => {
                if args.mode().is_expanded() {
                    let expanded_end = [Instruction::line(LineMode::Hard), Instruction::token(")")];

                    // finish the indent and print the hardline break and closing parentheses.
                    stack.pop(FormatTagKind::Indent)?;
                    indent_stack.pop();
                    queue.push_pair(expanded_end);
                }

                stack.pop(FormatTagKind::BestFitParenthesize)?;
            }

            DecodedInstruction::Tag(StartConditionalGroup(index)) => {
                let group = *self.document().group(index);
                let condition = group.condition();
                let expected_mode = match condition.group_id {
                    None => args.mode(),
                    Some(id) => self.state.group_modes.print_mode(id)?,
                };

                if expected_mode == condition.mode {
                    let print_mode = match group.mode() {
                        GroupMode::Expand | GroupMode::Propagated => PrintMode::Expanded,
                        GroupMode::Flat => self.flat_group_print_mode(
                            FormatTagKind::ConditionalGroup,
                            None,
                            args,
                            queue,
                            stack,
                            indent_stack,
                        )?,
                    };

                    stack.push(
                        FormatTagKind::ConditionalGroup,
                        args.with_print_mode(print_mode),
                    );
                } else {
                    // condition isn't met, render as normal content
                    stack.push(FormatTagKind::ConditionalGroup, args);
                }
            }

            DecodedInstruction::Tag(StartFill) => {
                self.print_fill_entries(queue, stack, indent_stack)?;
            }

            DecodedInstruction::Tag(StartIndent) => {
                indent_stack.indent(self.options.indent_style);
                stack.push(FormatTagKind::Indent, args);
            }

            DecodedInstruction::Tag(StartDedent(mode)) => {
                match mode {
                    DedentMode::Level => indent_stack.start_dedent(),
                    DedentMode::Root => indent_stack.reset_indent(),
                }
                stack.push(FormatTagKind::Dedent, args);
            }

            DecodedInstruction::Tag(StartAlign(align)) => {
                indent_stack.align(align);
                stack.push(FormatTagKind::Align, args);
            }

            DecodedInstruction::Tag(StartConditionalContent(Condition { mode, group_id })) => {
                let group_mode = match group_id {
                    None => args.mode(),
                    Some(id) => self.state.group_modes.print_mode(id)?,
                };

                if mode == group_mode {
                    stack.push(FormatTagKind::ConditionalContent, args);
                } else {
                    queue.skip_content(FormatTagKind::ConditionalContent)?;
                }
            }

            DecodedInstruction::Tag(StartIndentIfGroupBreaks(group_id)) => {
                let group_mode = self.state.group_modes.print_mode(group_id)?;

                if group_mode == PrintMode::Expanded {
                    indent_stack.indent(self.options.indent_style);
                }

                stack.push(FormatTagKind::IndentIfGroupBreaks, args);
            }

            DecodedInstruction::Tag(StartLineSuffix) => {
                indent_stack.push_suffix(indent_stack.indentation());
                self.state
                    .line_suffixes
                    .extend(args, queue.iter_content(FormatTagKind::LineSuffix))?;
            }

            DecodedInstruction::Tag(StartVerbatim(kind)) => {
                if let VerbatimKind::Verbatim { length } = kind {
                    #[expect(clippy::cast_possible_truncation)]
                    self.state.verbatim_markers.push(Span::at(
                        self.source.id,
                        self.state.buffer.len() as u32,
                        length,
                    ));
                }

                stack.push(FormatTagKind::Verbatim, args);
            }

            DecodedInstruction::Tag(StartFitsExpanded(index)) => {
                let condition = self.document().fits_expanded(index).condition;
                let condition_met = match condition {
                    Some(condition) => {
                        let group_mode = match condition.group_id {
                            Some(group_id) => self.state.group_modes.print_mode(group_id)?,
                            None => args.mode(),
                        };

                        condition.mode == group_mode
                    }
                    None => true,
                };

                if condition_met {
                    // require fresh measurement after selecting expanded nested groups
                    self.state.measured_group_fits = false;
                }

                stack.push(FormatTagKind::FitsExpanded, args);
            }

            DecodedInstruction::Tag(tag @ StartEntry) => {
                stack.push(tag.kind(), args);
            }

            DecodedInstruction::Tag(
                tag @ (EndEntry
                | EndGroup
                | EndConditionalGroup
                | EndConditionalContent
                | EndFitsExpanded
                | EndVerbatim
                | EndFill),
            ) => {
                stack.pop(tag.kind())?;
            }
            DecodedInstruction::Tag(tag @ EndIndentIfGroupBreaks(group_id)) => {
                if self.state.group_modes.print_mode(group_id)? == PrintMode::Expanded {
                    indent_stack.pop();
                }
                stack.pop(tag.kind())?;
            }
            DecodedInstruction::Tag(tag @ (EndIndent | EndAlign | EndLineSuffix)) => {
                stack.pop(tag.kind())?;
                indent_stack.pop();
            }
            DecodedInstruction::Tag(tag @ EndDedent(mode)) => {
                match mode {
                    DedentMode::Level => indent_stack.end_dedent(),
                    DedentMode::Root => indent_stack.pop(),
                }
                stack.pop(tag.kind())?;
            }
        }

        Ok(())
    }

    /// Return the document currently being printed.
    fn document(&self) -> &Document<'a> {
        debug_assert!(self.document.is_some());

        // safety: print_with_indent installs the document before processing instructions
        unsafe { self.document.unwrap_unchecked() }
    }

    /// Measure whether the remaining queue fits under the active arguments.
    fn fits(
        &mut self,
        queue: &PrintQueue<'a>,
        stack: &PrintCallStack,
        indent_stack: &PrintIndentStack,
    ) -> PrintResult<bool> {
        let mut measurer = FitsMeasurer::new(queue, stack, indent_stack, self);
        let result = measurer.fits(&mut AllPredicate);
        measurer.finish();

        result
    }

    /// Select the print mode for one initially flat group.
    fn flat_group_print_mode(
        &mut self,
        kind: FormatTagKind,
        id: Option<GroupId>,
        args: PrintArgs,
        queue: &PrintQueue<'a>,
        stack: &mut PrintCallStack,
        indent_stack: &PrintIndentStack,
    ) -> PrintResult<PrintMode> {
        let print_mode = match args.mode() {
            PrintMode::Flat if self.state.measured_group_fits => {
                // reuse the enclosing flat measurement
                PrintMode::Flat
            }
            // measure after entering expanded mode or crossing a printed line break
            _ => {
                self.state.measured_group_fits = true;

                if let Some(id) = id {
                    self.state.group_modes.set_print_mode(id, PrintMode::Flat);
                }

                // measure the group under flat arguments
                stack.push(kind, args.with_print_mode(PrintMode::Flat));
                let fits = self.fits(queue, stack, indent_stack)?;
                stack.pop(kind)?;

                if fits {
                    PrintMode::Flat
                } else {
                    PrintMode::Expanded
                }
            }
        };

        Ok(print_mode)
    }

    /// Print one static or borrowed text value.
    fn print_text(&mut self, text: Text<'_>) -> PrintResult<()> {
        self.write_pending_indent()?;

        self.push_marker();

        match text {
            #[expect(clippy::cast_possible_truncation)]
            Text::Token(token) => {
                self.state.push_str(token)?;
                self.state.line_width = self.state.line_width.saturating_add(token.len() as u32);
            }
            Text::Text {
                text,
                text_width: width,
            } => {
                if let Some(width) = width.width() {
                    self.state.push_str(text)?;
                    self.state.line_width = self.state.line_width.saturating_add(width.value());
                } else {
                    self.print_multiline_text(text)?;
                }
            }
        }

        Ok(())
    }

    /// Write pending indentation into the output buffer.
    fn write_pending_indent(&mut self) -> PrintResult<()> {
        let indent = std::mem::take(&mut self.state.pending_indent);
        if indent.is_empty() {
            return Ok(());
        }

        let level = indent.level() as usize;
        let align = indent.align() as usize;
        let indent_width = self.options.indent_width as usize;

        // write indentation in one buffer operation
        match self.options.indent_style {
            IndentStyle::Space => {
                let total_spaces = level.saturating_mul(indent_width).saturating_add(align);
                self.state.reserve(total_spaces)?;
                push_spaces(&mut self.state.buffer, total_spaces);
                self.state.line_width = self.state.line_width.saturating_add(total_spaces as u32);
            }
            IndentStyle::Tab => {
                let total_bytes = level.saturating_add(align);
                self.state.reserve(total_bytes)?;
                push_tabs(&mut self.state.buffer, level);
                push_spaces(&mut self.state.buffer, align);
                let display_width = level.saturating_mul(indent_width).saturating_add(align) as u32;
                self.state.line_width = self.state.line_width.saturating_add(display_width);
            }
        }

        Ok(())
    }

    /// Print multiline text.
    fn print_multiline_text(&mut self, text: &str) -> PrintResult<()> {
        // ascii fast path: stream chunks between newline and tab characters
        if text.is_ascii() {
            return self.print_multiline_ascii_text(text);
        }

        for char in text.chars() {
            self.print_char(char)?;
        }

        Ok(())
    }

    /// Print multiline ASCII text.
    fn print_multiline_ascii_text(&mut self, text: &str) -> PrintResult<()> {
        let bytes = text.as_bytes();
        let mut run_start = 0usize;
        let mut index = 0usize;

        while index < bytes.len() {
            let byte = bytes[index];

            if byte != b'\n' && byte != b'\t' {
                index += 1;
                continue;
            }

            if run_start < index {
                let chunk = &text[run_start..index];
                self.state.push_str(chunk)?;
                self.state.line_width = self
                    .state
                    .line_width
                    .saturating_add((index - run_start) as u32);
            }

            if byte == b'\n' {
                self.print_newline()?;
            } else {
                self.state.push_char('\t')?;
                self.state.line_width = self
                    .state
                    .line_width
                    .saturating_add(u32::from(self.options.indent_width));
            }

            index += 1;
            run_start = index;
        }

        if run_start < bytes.len() {
            let chunk = &text[run_start..];
            self.state.push_str(chunk)?;
            self.state.line_width = self
                .state
                .line_width
                .saturating_add((bytes.len() - run_start) as u32);
        }

        Ok(())
    }

    fn push_marker(&mut self) {
        let Some(source_position) = self.state.pending_source_position.take() else {
            return;
        };

        let marker = FileMarker {
            source: source_position,
            dest: self.state.buffer.len() as u32,
        };

        if self.state.source_markers.last() != Some(&marker) {
            self.state.source_markers.push(marker);
        }
    }

    /// Queue pending line suffixes and return whether any were queued.
    fn flush_line_suffixes(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        indent_stack: &mut PrintIndentStack,
        line_break: Option<Instruction<'a>>,
    ) -> bool {
        let suffixes = self.state.line_suffixes.take_pending();

        if suffixes.len() != 0 {
            // print this line break again once all line suffixes have been flushed
            if let Some(line_break) = line_break {
                queue.push_instruction(line_break);
            }

            indent_stack.flush_suffixes();
            for entry in suffixes.rev() {
                match entry {
                    LineSuffixEntry::Suffix(suffix) => {
                        queue.push_instruction(suffix);
                    }
                    LineSuffixEntry::Args(args) => {
                        stack.push(FormatTagKind::LineSuffix, args);
                        queue.push_instruction(Instruction::end_line_suffix());
                    }
                }
            }

            true
        } else {
            false
        }
    }

    fn print_best_fitting(
        &mut self,
        variants: BestFittingVariants<'a>,
        mode: BestFittingMode,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        indent_stack: &mut PrintIndentStack,
    ) -> PrintResult<()> {
        let args = stack.top();

        if args.mode().is_flat() && self.state.measured_group_fits {
            queue.push_slice(variants.most_flat());
            self.print_entry(queue, stack, indent_stack, args)
        } else {
            self.state.measured_group_fits = true;
            let mut variants_iter = variants.iter();
            let current = variants_iter.next();
            debug_assert!(current.is_some());
            // safety: every best-fitting instruction has at least two variants
            let mut current = unsafe { current.unwrap_unchecked() };

            for next in variants_iter {
                // try to fit only the first variant on a single line
                let mut current_instructions = current.iter();
                let start = current_instructions.next();
                if !start.is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry) {
                    return invalid_start_tag(FormatTagKind::Entry, start);
                }

                // skip the entry start because measurement supplies its print arguments
                let content = current_instructions.remaining();

                let entry_args = args
                    .with_print_mode(PrintMode::Flat)
                    .with_measure_mode(MeasureMode::from(mode));

                queue.push_slice(content);
                stack.push(FormatTagKind::Entry, entry_args);
                let variant_fits = self.fits(queue, stack, indent_stack)?;
                stack.pop(FormatTagKind::Entry)?;

                // remove measured content because printing needs the complete variant
                let measured = queue.pop_frame();
                debug_assert!(measured.is_some());

                if variant_fits {
                    queue.push_slice(current);
                    return self.print_entry(
                        queue,
                        stack,
                        indent_stack,
                        args.with_print_mode(PrintMode::Flat),
                    );
                }

                current = next;
            }

            // no variant fits, take the last (most expanded) as fallback
            queue.push_slice(current);
            self.print_entry(
                queue,
                stack,
                indent_stack,
                args.with_print_mode(PrintMode::Expanded),
            )
        }
    }

    /// Fit as many fill entries as possible on each line.
    ///
    /// `Fill` is a sequence of *item*, *separator*, *item*, *separator*, *item*, ... entries.
    /// The goal is to fit as many items (with their separators) on a single line as possible and
    /// first expand the *separator* if the content exceeds the print width and only fallback to expanding
    /// the *item*s if the *item* or the *item* and the expanded *separator* don't fit on the line.
    ///
    /// The implementation handles the following five cases:
    ///
    /// - The *item*, *separator*, and the *next item* fit on the same line.
    ///   Print the *item* and *separator* in flat mode.
    /// - The *item* and *separator* fit on the line but there's not enough space for the *next item*.
    ///   Print the *item* in flat mode and the *separator* in expanded mode.
    /// - The *item* fits on the line but the *separator* does not in flat mode.
    ///   Print the *item* in flat mode and the *separator* in expanded mode.
    /// - The *item* fits on the line but the *separator* does not in flat **NOR** expanded mode.
    ///   Print the *item* and *separator* in expanded mode.
    /// - The *item* does not fit on the line.
    ///   Print the *item* and *separator* in expanded mode.
    fn print_fill_entries(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        indent_stack: &mut PrintIndentStack,
    ) -> PrintResult<()> {
        let args = stack.top();

        // print all entries flat when an enclosing measurement already proved they fit
        if self.state.measured_group_fits && args.mode().is_flat() {
            stack.push(FormatTagKind::Fill, args.with_print_mode(PrintMode::Flat));
            return Ok(());
        }

        stack.push(FormatTagKind::Fill, args);

        while queue
            .peek()
            .is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry)
        {
            let mut measurer = FitsMeasurer::new_flat(queue, stack, indent_stack, self);

            // measure item and separator pairs that fit on the same line
            let mut flat_pairs = 0usize;
            let mut item_fits = measurer.fill_item_fits()?;

            let last_pair_layout = if item_fits {
                // measure pairs until the first item or separator that does not fit
                // avoid measuring the next item twice:
                // * once when measuring if the *item*, *separator*, *next-item* fit
                // * a second time when measuring if *next-item*, *separator*, *next-next-item* fit.
                loop {
                    // finish after an item without a following separator
                    if !measurer
                        .queue
                        .peek()
                        .is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry)
                    {
                        break FillPairLayout::Flat;
                    }

                    let separator_fits = measurer.fill_separator_fits(PrintMode::Flat)?;

                    // expand a separator that does not fit flat
                    if !separator_fits {
                        break FillPairLayout::ItemMaybeFlat;
                    }

                    // finish after the final item and separator pair
                    if !measurer
                        .queue
                        .peek()
                        .is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry)
                    {
                        break FillPairLayout::Flat;
                    }

                    item_fits = measurer.fill_item_fits()?;

                    if item_fits {
                        flat_pairs += 1;
                    } else {
                        // expand the separator before measuring the next item again
                        break FillPairLayout::ItemFlatSeparatorExpanded;
                    }
                }
            } else {
                // expand both an item and separator that do not fit
                FillPairLayout::Expanded
            };

            measurer.finish();

            // print all pairs proven to fit flat
            for _ in 0..flat_pairs {
                self.print_entry(
                    queue,
                    stack,
                    indent_stack,
                    args.with_print_mode(PrintMode::Flat),
                )?;
                self.print_entry(
                    queue,
                    stack,
                    indent_stack,
                    args.with_print_mode(PrintMode::Flat),
                )?;
            }

            let item_mode = match last_pair_layout {
                FillPairLayout::Flat | FillPairLayout::ItemFlatSeparatorExpanded => PrintMode::Flat,
                FillPairLayout::Expanded => PrintMode::Expanded,
                FillPairLayout::ItemMaybeFlat => {
                    let mut measurer = FitsMeasurer::new_flat(queue, stack, indent_stack, self);
                    // restore the measurement state before measuring the separator
                    if !measurer.fill_item_fits()? {
                        return Err(PrintError::UnstableLayout);
                    }
                    let separator_fits = measurer.fill_separator_fits(PrintMode::Expanded)?;
                    measurer.finish();

                    if separator_fits {
                        PrintMode::Flat
                    } else {
                        PrintMode::Expanded
                    }
                }
            };

            self.print_entry(queue, stack, indent_stack, args.with_print_mode(item_mode))?;

            if queue
                .peek()
                .is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry)
            {
                let separator_mode = match last_pair_layout {
                    FillPairLayout::Flat => PrintMode::Flat,
                    FillPairLayout::ItemFlatSeparatorExpanded
                    | FillPairLayout::Expanded
                    | FillPairLayout::ItemMaybeFlat => PrintMode::Expanded,
                };

                // keep nested separator groups flat while the separator itself expands
                stack.push(FormatTagKind::Fill, args.with_print_mode(PrintMode::Flat));
                self.print_entry(
                    queue,
                    stack,
                    indent_stack,
                    args.with_print_mode(separator_mode),
                )?;
                stack.pop(FormatTagKind::Fill)?;
            }
        }

        if queue
            .peek()
            .is_some_and(|instruction| instruction.opcode() == Opcode::EndFill)
        {
            Ok(())
        } else {
            invalid_end_tag(FormatTagKind::Fill, stack.top_kind())
        }
    }

    /// Print one complete entry and restore the surrounding queue state.
    fn print_entry(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        indent_stack: &mut PrintIndentStack,
        args: PrintArgs,
    ) -> PrintResult<()> {
        let kind = FormatTagKind::Entry;
        let start_entry = queue.peek();

        if queue
            .pop()
            .is_some_and(|start| start.tag_kind() == Some(kind))
        {
            stack.push(kind, args);
        } else {
            return invalid_start_tag(kind, start_entry);
        }

        let mut depth = 1u32;

        while let Some(instruction) = queue.pop() {
            match instruction.decode() {
                DecodedInstruction::Tag(InstructionTag::StartEntry) => {
                    depth += 1;
                }
                DecodedInstruction::Tag(end_tag @ InstructionTag::EndEntry) => {
                    depth -= 1;
                    // complete the outer entry
                    if depth == 0 {
                        stack.pop(end_tag.kind())?;
                        return Ok(());
                    }
                }
                _ => {
                    // fall through
                }
            }

            self.print_instruction(stack, indent_stack, queue, instruction)?;
        }

        invalid_end_tag(kind, stack.top_kind())
    }

    /// Print one character and update the current display width.
    fn print_char(&mut self, char: char) -> PrintResult<()> {
        if char == '\n' {
            self.print_newline()?;
        } else {
            self.state.push_char(char)?;

            let char_width = if char.is_ascii() {
                if char == '\t' {
                    self.options.indent_width as u32
                } else {
                    1
                }
            } else {
                u32::from(char.terminal_display_width())
            };

            self.state.line_width = self.state.line_width.saturating_add(char_width);
        }

        Ok(())
    }

    /// Trim trailing spaces and tabs from the current line.
    fn trim_trailing_line_whitespace(&mut self) {
        let line_start = self.state.line_start;
        let mut trimmed_len = self.state.buffer.len();

        while trimmed_len > line_start {
            let byte = self.state.buffer.as_bytes()[trimmed_len - 1];
            if matches!(byte, b' ' | b'\t') {
                trimmed_len -= 1;
            } else {
                break;
            }
        }

        if trimmed_len == self.state.buffer.len() {
            return;
        }

        self.state.buffer.truncate(trimmed_len);

        while self
            .state
            .source_markers
            .last()
            .is_some_and(|marker| marker.dest as usize > trimmed_len)
        {
            self.state.source_markers.pop();
        }

        for verbatim in self.state.verbatim_markers.iter_mut().rev() {
            if verbatim.end as usize <= trimmed_len {
                break;
            }

            if verbatim.start as usize >= trimmed_len {
                *verbatim = Span::new(verbatim.file, trimmed_len as u32, trimmed_len as u32);
            } else {
                verbatim.end = trimmed_len as u32;
            }
        }

        while self
            .state
            .verbatim_markers
            .last()
            .is_some_and(|range| range.start == range.end)
        {
            self.state.verbatim_markers.pop();
        }
    }

    /// Print a newline with the configured line ending.
    fn print_newline(&mut self) -> PrintResult<()> {
        if self.options.trim_trailing_whitespace {
            self.trim_trailing_line_whitespace();
        }

        self.state.push_str(self.options.line_ending.as_str())?;

        self.state.line_width = 0;
        self.state.line_start = self.state.buffer.len();

        // fit's only tests if groups up to the first line break fit.
        // the next group must re-measure if it still fits.
        self.state.measured_group_fits = false;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum FillPairLayout {
    /// The item, separator, and next item fit flat.
    Flat,
    /// The item and separator fit flat, but the next item requires expansion.
    ItemFlatSeparatorExpanded,
    /// The item requires expansion.
    Expanded,
    /// The item fits flat, while the separator requires another measurement.
    ItemMaybeFlat,
}

/// Mutable output and reusable measurement state for one print call.
#[derive(Default, Debug)]
struct PrinterState<'a> {
    /// The formatted output.
    buffer: String,

    /// The maximum formatted output bytes to emit.
    max_output_bytes: u32,

    /// The source markers that map source positions to formatted positions.
    source_markers: Vec<FileMarker>,

    /// The next source position that should be flushed when writing the next text.
    pending_source_position: Option<u32>,

    /// The current indentation that should be written before the next text.
    pending_indent: Indentation,

    /// Whether content through the next line break has already measured flat.
    measured_group_fits: bool,

    /// The offset at which the current line in `buffer` starts.
    line_start: usize,

    /// The accumulated unicode-width of all characters on the current line.
    line_width: u32,

    /// The line suffixes that should be printed at the end of the line.
    line_suffixes: LineSuffixes<'a>,
    /// The formatted ranges copied verbatim from source.
    verbatim_markers: Vec<Span>,
    /// The selected modes of externally referenced groups.
    group_modes: GroupModes,
    /// Reusable structural stack storage for fit measurement.
    fits_stack: Vec<StackFrame>,
    /// Reusable indentation stack storage for fit measurement.
    fits_indent_stack: Vec<Indentation>,
    /// Reusable dedentation history storage for fit measurement.
    fits_history_indent_stack: Vec<Indentation>,
    /// Reusable instruction queue storage for fit measurement.
    fits_queue: Vec<QueueFrame<'a>>,
}

impl PrinterState<'_> {
    /// Create output state with source-sized initial capacity.
    fn new(capacity: usize, max_output_bytes: u32) -> Self {
        let max_output_capacity = max_output_bytes as usize;

        Self {
            buffer: String::with_capacity(capacity.min(max_output_capacity)),
            max_output_bytes,
            ..Self::default()
        }
    }

    /// Reserve additional output bytes within the configured limit.
    fn reserve(&mut self, additional: usize) -> PrintResult<()> {
        self.check_output_len(additional)?;
        self.buffer.reserve(additional);

        Ok(())
    }

    /// Append text within the configured output limit.
    fn push_str(&mut self, text: &str) -> PrintResult<()> {
        self.check_output_len(text.len())?;
        self.buffer.push_str(text);

        Ok(())
    }

    /// Append one character within the configured output limit.
    fn push_char(&mut self, char: char) -> PrintResult<()> {
        self.check_output_len(char.len_utf8())?;
        self.buffer.push(char);

        Ok(())
    }

    /// Check one prospective output length increase.
    fn check_output_len(&self, additional: usize) -> PrintResult<()> {
        let Some(requested_bytes) = self.buffer.len().checked_add(additional) else {
            return Err(PrintError::OutputTooLarge {
                max_output_bytes: self.max_output_bytes,
                requested_bytes: RequestedOutputBytes::Overflow,
            });
        };

        if requested_bytes > self.max_output_bytes as usize {
            return Err(PrintError::OutputTooLarge {
                max_output_bytes: self.max_output_bytes,
                requested_bytes: RequestedOutputBytes::Count(requested_bytes),
            });
        }

        Ok(())
    }
}

/// Selected print modes indexed by dense document-local group identifiers.
#[derive(Debug, Default)]
struct GroupModes(Vec<Option<PrintMode>>);

impl GroupModes {
    /// Set the selected mode for one group.
    fn set_print_mode(&mut self, group_id: GroupId, mode: PrintMode) {
        let index = u32::from(group_id) as usize;

        if self.0.len() <= index {
            self.0.resize(index + 1, None);
        }

        self.0[index] = Some(mode);
    }

    /// Return the selected mode for one group.
    fn print_mode(&self, group_id: GroupId) -> PrintResult<PrintMode> {
        let index = u32::from(group_id) as usize;

        match self.0.get(index) {
            Some(Some(print_mode)) => Ok(*print_mode),
            None | Some(None) => Err(PrintError::InvalidDocument(
                InvalidDocumentError::UnknownGroupId { group_id },
            )),
        }
    }
}

/// One restorable simulation of the printer used for fit measurement.
struct FitsMeasurer<'a, 'print> {
    /// The current measurement state.
    state: FitsState,
    /// The restorable instruction queue.
    queue: FitsQueue<'a, 'print>,
    /// The restorable structural stack.
    stack: FitsCallStack<'print>,
    /// The restorable indentation stack.
    indent_stack: FitsIndentStack<'print>,
    /// The printer that owns reusable measurement storage.
    printer: &'print mut Printer<'a>,
    /// Whether every measured group must remain flat.
    must_be_flat: bool,
    /// Whether reusable storage has returned to the printer.
    is_restored: bool,
}

impl<'a, 'print> FitsMeasurer<'a, 'print> {
    /// Create one measurement that rejects expanded groups.
    fn new_flat(
        print_queue: &'print PrintQueue<'a>,
        print_stack: &'print PrintCallStack,
        print_indent_stack: &'print PrintIndentStack,
        printer: &'print mut Printer<'a>,
    ) -> Self {
        let mut measurer = Self::new(print_queue, print_stack, print_indent_stack, printer);
        measurer.must_be_flat = true;
        measurer
    }

    /// Create one measurement over the current printer stacks.
    fn new(
        print_queue: &'print PrintQueue<'a>,
        print_stack: &'print PrintCallStack,
        print_indent_stack: &'print PrintIndentStack,
        printer: &'print mut Printer<'a>,
    ) -> Self {
        let saved_stack = std::mem::take(&mut printer.state.fits_stack);
        let saved_indent_stack = std::mem::take(&mut printer.state.fits_indent_stack);
        let saved_history_indent_stack =
            std::mem::take(&mut printer.state.fits_history_indent_stack);
        let saved_queue = std::mem::take(&mut printer.state.fits_queue);
        debug_assert!(saved_stack.is_empty());
        debug_assert!(saved_indent_stack.is_empty());
        debug_assert!(saved_history_indent_stack.is_empty());
        debug_assert!(saved_queue.is_empty());

        let fits_queue = FitsQueue::new(print_queue, saved_queue);
        let fits_stack = FitsCallStack::new(print_stack, saved_stack);
        let fits_indent_stack = FitsIndentStack::new(
            print_indent_stack,
            saved_indent_stack,
            saved_history_indent_stack,
        );

        let fits_state = FitsState {
            pending_indent: printer.state.pending_indent,
            line_width: printer.state.line_width,
            has_line_suffix: printer.state.line_suffixes.has_pending(),
        };

        Self {
            state: fits_state,
            queue: fits_queue,
            stack: fits_stack,
            indent_stack: fits_indent_stack,
            must_be_flat: false,
            printer,
            is_restored: false,
        }
    }

    /// Measure through the predicate boundary without exceeding the line width.
    fn fits<P>(&mut self, predicate: &mut P) -> PrintResult<bool>
    where
        P: FitsEndPredicate,
    {
        while let Some(instruction) = self.queue.pop() {
            match self.fits_instruction(instruction)? {
                Fits::Yes => return Ok(true),
                Fits::No => {
                    return Ok(false);
                }
                Fits::Maybe => {
                    if predicate.is_end(instruction)? {
                        break;
                    }

                    continue;
                }
            }
        }

        Ok(true)
    }

    /// Measure one fill item in flat mode.
    fn fill_item_fits(&mut self) -> PrintResult<bool> {
        self.fill_entry_fits(PrintMode::Flat)
    }

    /// Measure one fill separator under the selected mode.
    fn fill_separator_fits(&mut self, mode: PrintMode) -> PrintResult<bool> {
        self.fill_entry_fits(mode)
    }

    /// Measure one complete fill entry under the selected mode.
    fn fill_entry_fits(&mut self, mode: PrintMode) -> PrintResult<bool> {
        let start_entry = self.queue.peek();

        if !start_entry.is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry) {
            return invalid_start_tag(FormatTagKind::Entry, start_entry);
        }

        self.stack
            .push(FormatTagKind::Fill, self.stack.top().with_print_mode(mode));
        let mut predicate = SingleEntryPredicate::default();
        let fits = self.fits(&mut predicate)?;

        if predicate.is_done() {
            self.stack.pop(FormatTagKind::Fill)?;
        }

        Ok(fits)
    }

    /// Measure whether one instruction fits on the current line.
    fn fits_instruction(&mut self, instruction: Instruction<'a>) -> PrintResult<Fits> {
        #[allow(clippy::enum_glob_use)]
        use InstructionTag::*;

        let args = self.stack.top();

        match instruction.decode() {
            DecodedInstruction::Space => return Ok(self.fits_text(Text::Token(" "), args)),

            DecodedInstruction::Line(line_mode) => {
                match args.mode() {
                    PrintMode::Flat => match line_mode {
                        LineMode::SoftOrSpace => return Ok(self.fits_text(Text::Token(" "), args)),
                        LineMode::Soft => {}
                        LineMode::Hard | LineMode::Empty => {
                            return Ok(Fits::Yes);
                        }
                    },
                    PrintMode::Expanded => {
                        match args.measure_mode() {
                            MeasureMode::FirstLine => {
                                // retain expanded mode after crossing the measured scope
                                // is what the mode's initialized to by default
                                // preserve expansion for following instructions outside this scope
                                // line break should be printed as regular line break
                                return Ok(Fits::Yes);
                            }
                            MeasureMode::AllLines | MeasureMode::AllLinesAllowTextOverflow => {
                                // Continue measuring on the next line
                                self.state.line_width = 0;
                                self.state.pending_indent = self.indent_stack.indentation();
                            }
                        }
                    }
                }
            }

            DecodedInstruction::Token(text) => return Ok(self.fits_text(Text::Token(text), args)),
            DecodedInstruction::Text {
                text,
                width: text_width,
            } => {
                return Ok(self.fits_text(Text::Text { text, text_width }, args));
            }
            DecodedInstruction::SourcePosition(_) => {}
            DecodedInstruction::FileSlice {
                range,
                width: text_width,
            } => {
                let text = self.printer.source.get_range_str(range).ok_or_else(|| {
                    PrintError::SourceTextUnavailable {
                        span: Span::new(self.printer.source.id, range.start, range.end),
                    }
                })?;
                return Ok(self.fits_text(Text::Text { text, text_width }, args));
            }
            DecodedInstruction::LineSuffixBoundary => {
                if self.state.has_line_suffix {
                    return Ok(Fits::No);
                }
            }

            DecodedInstruction::ExpandParent => {
                if self.must_be_flat {
                    return Ok(Fits::No);
                }
            }

            DecodedInstruction::BestFitting { variants, mode } => {
                let slice = match args.mode() {
                    PrintMode::Flat => (
                        variants.most_flat(),
                        args.with_measure_mode(MeasureMode::from(mode)),
                    ),
                    PrintMode::Expanded => (variants.most_expanded(), args),
                }
                .0;

                let start = slice.iter().next();
                if !start.is_some_and(|instruction| instruction.opcode() == Opcode::StartEntry) {
                    return invalid_start_tag(FormatTagKind::Entry, start);
                }

                self.queue.push_slice(slice);
            }

            DecodedInstruction::Slice(content) => self.queue.push_slice(content),

            DecodedInstruction::Tag(StartIndent) => {
                self.indent_stack.indent(self.options().indent_style);
                self.stack.push(FormatTagKind::Indent, args);
            }

            DecodedInstruction::Tag(StartDedent(mode)) => {
                match mode {
                    DedentMode::Level => self.indent_stack.start_dedent(),
                    DedentMode::Root => self.indent_stack.reset_indent(),
                }
                self.stack.push(FormatTagKind::Dedent, args);
            }

            DecodedInstruction::Tag(StartAlign(align)) => {
                self.indent_stack.align(align);
                self.stack.push(FormatTagKind::Align, args);
            }

            DecodedInstruction::Tag(StartGroup(index, _)) => {
                let group = *self.printer.document().group(index);

                return Ok(self.fits_group(FormatTagKind::Group, group.mode(), group.id(), args));
            }

            DecodedInstruction::Tag(StartBestFitParenthesize(id)) => {
                if let Some(id) = id {
                    self.printer
                        .state
                        .group_modes
                        .set_print_mode(id, args.mode());
                }

                // Don't use the parenthesized with indent layout even when measuring expanded mode similar to `BestFitting`.
                // This is to expand the left and not right after the `(` parentheses (it is okay to expand after the content that it wraps).
                self.stack.push(FormatTagKind::BestFitParenthesize, args);
            }

            DecodedInstruction::Tag(EndBestFitParenthesize) => {
                // If this is the end tag of the outer most parentheses for which we measure if it fits,
                // pop the indent.
                if args.mode().is_expanded() && self.stack.top_kind() == Some(FormatTagKind::Indent)
                {
                    self.stack.pop(FormatTagKind::Indent)?;
                    self.indent_stack.pop();
                    let unindented = self.stack.pop(FormatTagKind::BestFitParenthesize)?;

                    // There's a hard line break after the indent but don't return `Fits::Yes` here
                    // to ensure any trailing comments (that, unfortunately, are attached to the statement and not the expression)
                    // fit too.
                    self.state.line_width = 0;
                    self.state.pending_indent = self.indent_stack.indentation();

                    return Ok(self.fits_text(Text::Token(")"), unindented));
                }

                self.stack.pop(FormatTagKind::BestFitParenthesize)?;
            }

            DecodedInstruction::Tag(StartConditionalGroup(index)) => {
                let group = *self.printer.document().group(index);
                let condition = group.condition();

                let print_mode = match condition.group_id {
                    None => args.mode(),
                    Some(group_id) => self.group_modes().print_mode(group_id)?,
                };

                if condition.mode == print_mode {
                    return Ok(self.fits_group(
                        FormatTagKind::ConditionalGroup,
                        group.mode(),
                        None,
                        args,
                    ));
                }
                self.stack.push(FormatTagKind::ConditionalGroup, args);
            }

            DecodedInstruction::Tag(StartConditionalContent(condition)) => {
                let print_mode = match condition.group_id {
                    None => args.mode(),
                    Some(group_id) => self.group_modes().print_mode(group_id)?,
                };

                if condition.mode == print_mode {
                    self.stack.push(FormatTagKind::ConditionalContent, args);
                } else {
                    self.queue.skip_content(FormatTagKind::ConditionalContent)?;
                }
            }

            DecodedInstruction::Tag(StartIndentIfGroupBreaks(id)) => {
                let print_mode = self.group_modes().print_mode(id)?;

                if print_mode == PrintMode::Expanded {
                    self.indent_stack.indent(self.options().indent_style);
                }
                self.stack.push(FormatTagKind::IndentIfGroupBreaks, args);
            }

            DecodedInstruction::Tag(StartLineSuffix) => {
                self.queue.skip_content(FormatTagKind::LineSuffix)?;
                self.state.has_line_suffix = true;
            }

            DecodedInstruction::Tag(EndLineSuffix) => {
                return invalid_end_tag(FormatTagKind::LineSuffix, self.stack.top_kind());
            }

            DecodedInstruction::Tag(StartFitsExpanded(index)) => {
                let fits_expanded = *self.printer.document().fits_expanded(index);
                match args.mode() {
                    PrintMode::Expanded => {
                        // As usual, nothing to measure
                        self.stack.push(FormatTagKind::FitsExpanded, args);
                    }
                    PrintMode::Flat => {
                        let condition_met = match fits_expanded.condition {
                            Some(condition) => {
                                let group_mode = match condition.group_id {
                                    Some(group_id) => self.group_modes().print_mode(group_id)?,
                                    None => args.mode(),
                                };

                                condition.mode == group_mode
                            }
                            None => true,
                        };

                        if condition_met {
                            // Measure in fully expanded mode and allow overflows
                            self.stack.push(
                                FormatTagKind::FitsExpanded,
                                args.with_measure_mode(MeasureMode::AllLinesAllowTextOverflow)
                                    .with_print_mode(PrintMode::Expanded),
                            );
                        } else {
                            if fits_expanded.is_expanded {
                                return Ok(Fits::No);
                            }

                            // As usual
                            self.stack.push(FormatTagKind::FitsExpanded, args);
                        }
                    }
                }
            }

            DecodedInstruction::Tag(tag @ (StartFill | StartVerbatim(_) | StartEntry)) => {
                self.stack.push(tag.kind(), args);
            }

            DecodedInstruction::Tag(
                tag @ (EndFill
                | EndVerbatim
                | EndEntry
                | EndGroup
                | EndConditionalGroup
                | EndConditionalContent
                | EndFitsExpanded),
            ) => {
                self.stack.pop(tag.kind())?;
            }
            DecodedInstruction::Tag(tag @ EndIndentIfGroupBreaks(id)) => {
                if self.group_modes().print_mode(id)? == PrintMode::Expanded {
                    self.indent_stack.pop();
                }
                self.stack.pop(tag.kind())?;
            }
            DecodedInstruction::Tag(tag @ (EndIndent | EndAlign)) => {
                self.stack.pop(tag.kind())?;
                self.indent_stack.pop();
            }
            DecodedInstruction::Tag(tag @ EndDedent(mode)) => {
                match mode {
                    DedentMode::Level => self.indent_stack.end_dedent(),
                    DedentMode::Root => self.indent_stack.pop(),
                }
                self.stack.pop(tag.kind())?;
            }
        }

        Ok(Fits::Maybe)
    }

    /// Measure one group and enter its structural scope.
    fn fits_group(
        &mut self,
        kind: FormatTagKind,
        group_mode: GroupMode,
        id: Option<GroupId>,
        args: PrintArgs,
    ) -> Fits {
        if self.must_be_flat && !group_mode.is_flat() {
            return Fits::No;
        }

        // continue expanded groups while measuring a best fitting layout where
        // a group expands.
        let print_mode = if group_mode.is_flat() {
            args.mode()
        } else {
            PrintMode::Expanded
        };

        self.stack.push(kind, args.with_print_mode(print_mode));

        if let Some(id) = id {
            self.group_modes_mut().set_print_mode(id, print_mode);
        }

        Fits::Maybe
    }

    /// Measure one static or borrowed text value.
    fn fits_text(&mut self, text: Text<'_>, args: PrintArgs) -> Fits {
        let indent = std::mem::take(&mut self.state.pending_indent);
        if !indent.is_empty() {
            let indent_width = u32::from(indent.level())
                .saturating_mul(u32::from(self.options().indent_width))
                .saturating_add(u32::from(indent.align()));
            self.state.line_width = self.state.line_width.saturating_add(indent_width);
        }

        match text {
            #[expect(clippy::cast_possible_truncation)]
            Text::Token(token) => {
                self.state.line_width = self.state.line_width.saturating_add(token.len() as u32);
            }
            Text::Text { text, text_width } => {
                if let Some(width) = text_width.width() {
                    self.state.line_width = self.state.line_width.saturating_add(width.value());
                } else if text.is_ascii() {
                    let ascii_fit = self.fits_multiline_ascii_text(text, args);
                    if !matches!(ascii_fit, Fits::Maybe) {
                        return ascii_fit;
                    }
                } else {
                    for c in text.chars() {
                        let char_width = match c {
                            '\t' => self.options().indent_width,
                            '\n' => {
                                if self.must_be_flat {
                                    return Fits::No;
                                }
                                match args.measure_mode() {
                                    MeasureMode::FirstLine => {
                                        return if self.exceeds_width(args) {
                                            Fits::No
                                        } else {
                                            Fits::Yes
                                        };
                                    }
                                    MeasureMode::AllLines
                                    | MeasureMode::AllLinesAllowTextOverflow => {
                                        self.state.line_width = 0;
                                        continue;
                                    }
                                }
                            }
                            character => character.terminal_display_width(),
                        };
                        self.state.line_width =
                            self.state.line_width.saturating_add(u32::from(char_width));
                    }
                }
            }
        }

        if self.exceeds_width(args) {
            return Fits::No;
        }

        Fits::Maybe
    }

    /// Return whether the measured line exceeds the configured width.
    fn exceeds_width(&self, args: PrintArgs) -> bool {
        self.state.line_width > self.options().line_width.into()
            && !args.measure_mode().allows_text_overflow()
    }

    /// Measure ASCII text without decoding individual characters.
    fn fits_multiline_ascii_text(&mut self, text: &str, args: PrintArgs) -> Fits {
        let bytes = text.as_bytes();
        let mut run_start = 0usize;
        let mut index = 0usize;

        while index < bytes.len() {
            let byte = bytes[index];

            if byte != b'\n' && byte != b'\t' {
                index += 1;
                continue;
            }

            self.state.line_width = self
                .state
                .line_width
                .saturating_add((index - run_start) as u32);

            if byte == b'\n' {
                if self.must_be_flat {
                    return Fits::No;
                }

                match args.measure_mode() {
                    MeasureMode::FirstLine => {
                        return if self.exceeds_width(args) {
                            Fits::No
                        } else {
                            Fits::Yes
                        };
                    }
                    MeasureMode::AllLines | MeasureMode::AllLinesAllowTextOverflow => {
                        self.state.line_width = 0;
                    }
                }
            } else {
                self.state.line_width = self
                    .state
                    .line_width
                    .saturating_add(u32::from(self.options().indent_width));
            }

            index += 1;
            run_start = index;
        }

        self.state.line_width = self
            .state
            .line_width
            .saturating_add((bytes.len() - run_start) as u32);
        Fits::Maybe
    }

    /// Return reusable measurement storage to the printer.
    fn finish(mut self) {
        self.restore();
    }

    /// Restore reusable measurement storage to the printer.
    fn restore(&mut self) {
        if self.is_restored {
            return;
        }

        // restore queue storage
        let mut queue = self.queue.take_storage();
        queue.clear();
        self.printer.state.fits_queue = queue;

        // restore structural stack storage
        let mut stack = self.stack.take_storage();
        stack.clear();
        self.printer.state.fits_stack = stack;

        // restore indentation stack storage
        let (mut indent_stack, mut history_indent_stack) = self.indent_stack.take_storage();
        indent_stack.clear();
        history_indent_stack.clear();
        self.printer.state.fits_indent_stack = indent_stack;
        self.printer.state.fits_history_indent_stack = history_indent_stack;

        self.is_restored = true;
    }

    /// Return the active print options.
    fn options(&self) -> &PrintOptions {
        &self.printer.options
    }

    /// Return the selected group modes.
    fn group_modes(&self) -> &GroupModes {
        &self.printer.state.group_modes
    }

    /// Return the selected group modes mutably.
    fn group_modes_mut(&mut self) -> &mut GroupModes {
        &mut self.printer.state.group_modes
    }
}

impl Drop for FitsMeasurer<'_, '_> {
    fn drop(&mut self) {
        self.restore();
    }
}

#[cold]
pub(crate) fn invalid_end_tag<R>(
    end_tag: FormatTagKind,
    start_tag: Option<FormatTagKind>,
) -> PrintResult<R> {
    Err(PrintError::InvalidDocument(match start_tag {
        None => InvalidDocumentError::StartTagMissing { kind: end_tag },
        Some(start_tag) => InvalidDocumentError::StartEndTagMismatch {
            start_kind: start_tag,
            end_kind: end_tag,
        },
    }))
}

#[cold]
pub(crate) fn missing_end_tag<R>(kind: FormatTagKind) -> PrintResult<R> {
    Err(PrintError::InvalidDocument(
        InvalidDocumentError::EndTagMissing { kind },
    ))
}

#[cold]
pub(crate) fn invalid_start_tag<R>(
    expected: FormatTagKind,
    actual: Option<Instruction<'_>>,
) -> PrintResult<R> {
    let start = match actual.map(Instruction::decode) {
        None => ActualStart::EndOfDocument,
        Some(DecodedInstruction::Tag(tag)) => {
            if tag.is_start() {
                ActualStart::Start(tag.kind())
            } else {
                ActualStart::End(tag.kind())
            }
        }
        Some(_) => ActualStart::Content,
    };

    Err(PrintError::InvalidDocument(
        InvalidDocumentError::ExpectedStart {
            actual: start,
            expected_start: expected,
        },
    ))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Fits {
    /// The measured content fits.
    Yes,
    /// The measured content does not fit.
    No,
    /// The result depends on following instructions.
    Maybe,
}

impl From<bool> for Fits {
    fn from(value: bool) -> Self {
        if value { Fits::Yes } else { Fits::No }
    }
}

/// Push `count` spaces into a string buffer.
fn push_spaces(buffer: &mut String, count: usize) {
    const SPACES_CHUNK: &str = "                                                                ";
    let mut remaining = count;
    while remaining >= SPACES_CHUNK.len() {
        buffer.push_str(SPACES_CHUNK);
        remaining -= SPACES_CHUNK.len();
    }
    if remaining > 0 {
        buffer.push_str(&SPACES_CHUNK[..remaining]);
    }
}

/// Push `count` tabs into a string buffer.
fn push_tabs(buffer: &mut String, count: usize) {
    const TABS_CHUNK: &str = "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";
    let mut remaining = count;
    while remaining >= TABS_CHUNK.len() {
        buffer.push_str(TABS_CHUNK);
        remaining -= TABS_CHUNK.len();
    }
    if remaining > 0 {
        buffer.push_str(&TABS_CHUNK[..remaining]);
    }
}

/// Mutable state for one fit measurement.
#[derive(Debug)]
struct FitsState {
    /// The indentation pending before the next text.
    pending_indent: Indentation,
    /// Whether a line suffix occurred during measurement.
    has_line_suffix: bool,
    /// The measured width of the current line.
    line_width: u32,
}

#[derive(Copy, Clone, Debug)]
enum Text<'a> {
    /// ASCII only text that contains no line breaks or tab characters.
    Token(&'a str),
    /// Arbitrary text. May contain `\n` line breaks, tab characters, or unicode characters.
    Text {
        text: &'a str,
        text_width: TextWidth,
    },
}

#[cfg(test)]
mod tests {
    use tspp_source::{File, FileId, FileType, Uri};

    use crate::format::{
        Condition, Document, FormatElement, FormatState, IndentStyle, InvalidDocumentError,
        LineEnding, PrintError, RequestedOutputBytes, TextWidth,
    };
    use crate::prelude::*;
    use crate::print::{PrintOptions, Printed, Printer, invalid_end_tag};
    use crate::{format_args, write};

    fn print<'a>(allocator: &'a Allocator, root: &dyn Format<'a, SimpleFormatContext>) -> Printed {
        print_with_options(allocator, root, PrintOptions::default())
    }

    fn print_with_options<'a>(
        allocator: &'a Allocator,
        root: &dyn Format<'a, SimpleFormatContext>,
        options: PrintOptions,
    ) -> Printed {
        let formatted =
            crate::format!(allocator, SimpleFormatContext::empty_destack(), [root]).unwrap();

        Printer::new(&File::empty_text(FileType::Tspp), options)
            .print(formatted.document())
            .expect("Document to be valid")
    }

    /// Output limits should fail before writing a too-large token.
    #[test]
    fn test_limits_token_output_bytes() {
        let allocator = Allocator::default();
        let formatted = crate::format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [token("abcdef")]
        )
        .unwrap();
        let options = PrintOptions::default().with_max_output_bytes(3);

        let error = Printer::new(&File::empty_text(FileType::Tspp), options)
            .print(formatted.document())
            .unwrap_err();

        assert_eq!(
            error,
            PrintError::OutputTooLarge {
                max_output_bytes: 3,
                requested_bytes: RequestedOutputBytes::Count(6)
            }
        );
    }

    /// Report conditional content without its matching end tag.
    #[test]
    fn test_reports_missing_conditional_content_end() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Tag(FormatTag::StartConditionalContent(
            Condition::if_fits_on_line(),
        )));
        formatter.write_element(FormatElement::Token { text: "content" });
        let instructions = formatter.into_tape().into_slice();
        let (_, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);

        let error = Printer::new(&File::empty_text(FileType::Tspp), PrintOptions::default())
            .print(&document)
            .unwrap_err();

        assert_eq!(
            error,
            PrintError::InvalidDocument(InvalidDocumentError::EndTagMissing {
                kind: FormatTagKind::ConditionalContent,
            })
        );
    }

    /// Report mismatched structural kinds in start then end order.
    #[test]
    fn test_reports_mismatched_end_tag() {
        let error =
            invalid_end_tag::<()>(FormatTagKind::Group, Some(FormatTagKind::Indent)).unwrap_err();

        assert_eq!(
            error,
            PrintError::InvalidDocument(InvalidDocumentError::StartEndTagMismatch {
                start_kind: FormatTagKind::Indent,
                end_kind: FormatTagKind::Group,
            })
        );
    }

    /// Output limits should apply to generated indentation too.
    #[test]
    fn test_limits_indent_output_bytes() {
        let allocator = Allocator::default();
        let formatted = crate::format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [token("a"), block_indent(&token("b"))]
        )
        .unwrap();
        let options = PrintOptions::default().with_max_output_bytes(3);

        let error = Printer::new(&File::empty_text(FileType::Tspp), options)
            .print(formatted.document())
            .unwrap_err();

        assert!(matches!(
            error,
            PrintError::OutputTooLarge {
                max_output_bytes: 3,
                ..
            }
        ));
    }

    /// File slices require source text during printing.
    #[test]
    fn test_print_source_text_slice_reports_binary_source() {
        let file_id = FileId::from_logical_str("binary.bin");
        let file = File::from_binary(
            file_id,
            "binary.bin".to_string(),
            Uri::from_string("binary.bin"),
            None,
            FileType::Binary,
            vec![1, 2, 3],
        )
        .expect("test binary source should load");
        let span = Span::new(file_id, 0, 1);
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::FileSlice {
            range: span.range(),
            width: TextWidth::from_text("a", 4),
        });
        let instructions = formatter.into_tape().into_slice();
        let (_, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);

        let error = Printer::new(&file, PrintOptions::default())
            .print(&document)
            .unwrap_err();

        assert_eq!(error, PrintError::SourceTextUnavailable { span });
    }

    /// Groups that fit within line width should print on one line.
    #[test]
    fn test_prints_group_on_single_line_if_fits() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &FormatArrayNodes {
                items: vec![
                    &token("\"a\""),
                    &token("\"b\""),
                    &token("\"c\""),
                    &token("\"d\""),
                ],
            },
        );

        assert_eq!(r#"["a", "b", "c", "d"]"#, result.as_str());
    }

    /// Nested indentation should accumulate correctly.
    #[test]
    fn test_tracks_indent_for_each_token() {
        let allocator = Allocator::default();
        let formatted = print(
            &allocator,
            &format_args!(
                token("a"),
                soft_block_indent(&format_args!(
                    token("b"),
                    soft_block_indent(&format_args!(
                        token("c"),
                        soft_block_indent(&format_args!(token("d"), soft_line_break(), token("d"))),
                        token("c"),
                    )),
                    token("b"),
                )),
                token("a")
            ),
        );

        assert_eq!(
            "a
    b
        c
            d
            d
        c
    b
a",
            formatted.as_str()
        );
    }

    /// Line endings should be converted according to options.
    #[test]
    fn test_converts_line_endings() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            line_ending: LineEnding::CarriageReturnLineFeed,
            ..PrintOptions::default()
        };

        let result = print_with_options(
            &allocator,
            &format_args![
                token("function main() {"),
                block_indent(&text("let x = `This is a multiline\nstring`;")),
                token("}"),
                hard_line_break()
            ],
            options,
        );

        assert_eq!(
            "function main() {\r\n    let x = `This is a multiline\r\nstring`;\r\n}\r\n",
            result.as_str()
        );
    }

    /// Trailing spaces should remain when trim trailing whitespace is disabled.
    #[test]
    fn test_preserves_trailing_whitespace_before_newline_when_disabled() {
        let allocator = Allocator::default();
        let result = print_with_options(
            &allocator,
            &format_args![
                token("a  "),
                hard_line_break(),
                token("b "),
                hard_line_break(),
                token("c  "),
                hard_line_break(),
            ],
            PrintOptions::default().with_trim_trailing_whitespace(false),
        );

        assert_eq!("a  \nb \nc  \n", result.as_str());
    }

    /// Trailing spaces should be trimmed before newline when trim trailing whitespace is enabled.
    #[test]
    fn test_trims_trailing_whitespace_before_newline_when_enabled() {
        let allocator = Allocator::default();
        let result = print_with_options(
            &allocator,
            &format_args![
                token("a  "),
                hard_line_break(),
                token("b "),
                hard_line_break(),
                token("c  "),
                hard_line_break(),
            ],
            PrintOptions::default().with_trim_trailing_whitespace(true),
        );

        assert_eq!("a\nb\nc\n", result.as_str());
    }

    /// Trailing tab characters should be trimmed before newline when trim trailing whitespace is enabled.
    #[test]
    fn test_trims_trailing_tab_before_newline_when_enabled() {
        let allocator = Allocator::default();
        let result = print_with_options(
            &allocator,
            &format_args![text("a\t"), hard_line_break()],
            PrintOptions::default().with_trim_trailing_whitespace(true),
        );

        assert_eq!("a\n", result.as_str());
    }

    /// Groups containing strings with newlines should break.
    #[test]
    fn test_breaks_group_if_string_contains_newline() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &FormatArrayNodes {
                items: vec![
                    &copied_text("`This is a string spanning\ntwo lines`"),
                    &token("\"b\""),
                ],
            },
        );

        assert_eq!(
            r#"[
    `This is a string spanning
two lines`,
    "b",
]"#,
            result.as_str()
        );
    }

    /// Groups with hard line breaks should always break.
    #[test]
    fn test_breaks_group_if_contains_hard_line_break() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &group(&format_args![token("a"), block_indent(&token("b"))]),
        );

        assert_eq!("a\n    b\n", result.as_str());
    }

    /// Parent groups should break when child content doesn't fit.
    #[test]
    fn test_breaks_parent_groups_if_dont_fit_on_single_line() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = print_with_options(
            &allocator,
            &FormatArrayNodes {
                items: vec![
                    &token("\"a\""),
                    &token("\"b\""),
                    &token("\"c\""),
                    &token("\"d\""),
                    &FormatArrayNodes {
                        items: vec![
                            &token("\"0123456789\""),
                            &token("\"0123456789\""),
                            &token("\"0123456789\""),
                            &token("\"0123456789\""),
                            &token("\"0123456789\""),
                        ],
                    },
                ],
            },
            options,
        );

        assert_eq!(
            r#"[
    "a",
    "b",
    "c",
    "d",
    ["0123456789", "0123456789", "0123456789", "0123456789", "0123456789"],
]"#,
            result.as_str()
        );
    }

    /// Groups should account for trailing statement content when deciding whether they fit.
    #[test]
    fn test_group_measurement_includes_following_statement_suffix() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = print_with_options(
            &allocator,
            &format_args![
                token("expect(genCode(createVNodeCall(null, \"`div`\", mockProps)))"),
                group(&indent(&format_args![
                    soft_line_break(),
                    token(".toMatchInlineSnapshot")
                ])),
                token(";")
            ],
            options,
        );

        assert_eq!(
            r#"expect(genCode(createVNodeCall(null, "`div`", mockProps)))
    .toMatchInlineSnapshot;"#,
            result.as_str()
        );
    }

    /// Groups should account for a following multiline call suffix when deciding whether they fit.
    #[test]
    fn test_group_measurement_includes_following_multiline_call_suffix() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = print_with_options(
            &allocator,
            &format_args![
                token("expect(genCode(createVNodeCall(null, \"`div`\", mockProps)))"),
                group(&indent(&format_args![
                    soft_line_break(),
                    token(".toMatchInlineSnapshot")
                ])),
                token("("),
                text("`\n  `"),
                token(")")
            ],
            options,
        );

        assert_eq!(
            "expect(genCode(createVNodeCall(null, \"`div`\", mockProps)))\n    .toMatchInlineSnapshot(`\n  `)",
            result.as_str()
        );
    }

    /// Indentation should use the character specified in options.
    #[test]
    fn test_uses_indent_character_from_options() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            indent_width: 4,
            line_width: 19,
            indent_style: IndentStyle::Tab,
            ..PrintOptions::default()
        };
        let result = print_with_options(
            &allocator,
            &FormatArrayNodes {
                items: vec![&token("'a'"), &token("'b'"), &token("'c'"), &token("'d'")],
            },
            options,
        );
        assert_eq!("[\n\t'a',\n\t\'b',\n\t\'c',\n\t'd',\n]", result.as_str());
    }

    /// Multiple consecutive hard line breaks should collapse to one.
    #[test]
    fn test_prints_consecutive_hard_lines_as_one() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &format_args![
                token("a"),
                hard_line_break(),
                hard_line_break(),
                hard_line_break(),
                token("b"),
            ],
        );

        assert_eq!("a\nb", result.as_str());
    }

    /// Empty lines should not collapse.
    #[test]
    fn test_prints_consecutive_empty_lines_as_many() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &format_args![
                token("a"),
                empty_line(),
                empty_line(),
                empty_line(),
                token("b"),
            ],
        );

        assert_eq!("a\n\n\n\nb", result.as_str());
    }

    /// Mixed empty lines and hard breaks should preserve empty lines.
    #[test]
    fn test_prints_consecutive_mixed_lines_as_many() {
        let allocator = Allocator::default();
        let result = print(
            &allocator,
            &format_args![
                token("a"),
                empty_line(),
                hard_line_break(),
                empty_line(),
                hard_line_break(),
                token("b"),
            ],
        );

        assert_eq!("a\n\n\nb", result.as_str());
    }

    /// Fill should break items optimally based on line width.
    #[test]
    fn test_fill_breaks() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut formatter = Formatter::new(&mut state);

        formatter
            .fill()
            // These all fit on the same line together
            .entry(
                &soft_line_break_or_space(),
                &format_args!(token("1"), token(",")),
            )
            .entry(
                &soft_line_break_or_space(),
                &format_args!(token("2"), token(",")),
            )
            .entry(
                &soft_line_break_or_space(),
                &format_args!(token("3"), token(",")),
            )
            // This one fits on a line by itself,
            .entry(
                &soft_line_break_or_space(),
                &format_args!(token("723493294"), token(",")),
            )
            // fits without breaking
            .entry(
                &soft_line_break_or_space(),
                &group(&format_args!(
                    token("["),
                    soft_block_indent(&token("5")),
                    token("],")
                )),
            )
            // this one must be printed in expanded mode to fit
            .entry(
                &soft_line_break_or_space(),
                &group(&format_args!(
                    token("["),
                    soft_block_indent(&token("123456789")),
                    token("]"),
                )),
            )
            .finish()
            .unwrap();

        let instructions = formatter.into_tape().into_slice();
        let (_, groups, fits_expanded) = state.finish();
        let document = Document::new(instructions, groups, fits_expanded);

        let printed = Printer::new(
            &File::empty_text(FileType::Tspp),
            PrintOptions::default().with_line_width(10),
        )
        .print(&document)
        .unwrap();

        assert_eq!(
            printed.as_str(),
            "1, 2, 3,\n723493294,\n[5],\n[\n    123456789\n]"
        );
    }

    /// Line suffixes should appear at the end of their line.
    #[test]
    fn test_line_suffix_printed_at_end() {
        let allocator = Allocator::default();
        let printed = print(
            &allocator,
            &format_args![
                group(&format_args![
                    token("["),
                    soft_block_indent(&format_with(|f| {
                        f.fill()
                            .entry(
                                &soft_line_break_or_space(),
                                &format_args!(token("1"), token(",")),
                            )
                            .entry(
                                &soft_line_break_or_space(),
                                &format_args!(token("2"), token(",")),
                            )
                            .entry(
                                &soft_line_break_or_space(),
                                &format_args!(token("3"), if_group_breaks(&token(","))),
                            )
                            .finish()
                    })),
                    token("]")
                ]),
                token(";"),
                line_suffix(&format_args![space(), token("// trailing")])
            ],
        );

        assert_eq!(printed.as_str(), "[1, 2, 3]; // trailing");
    }

    /// Conditional formatting should work correctly with group IDs.
    #[test]
    fn test_conditional_with_group_id_in_fits() {
        let allocator = Allocator::default();
        let content = format_with(|f| {
            let group_id = f.group_id();
            write!(
                f,
                [
                    group(&format_args![
                        token("The referenced group breaks."),
                        hard_line_break()
                    ])
                    .with_id(Some(group_id)),
                    group(&format_args![
                        token("This group breaks because:"),
                        soft_line_break_or_space(),
                        if_group_fits_on_line(&token("This content fits but should not be printed.")).with_group_id(Some(group_id)),
                        if_group_breaks(&token("It measures with the 'if_group_breaks' variant because the referenced group breaks and that's just way too much text.")).with_group_id(Some(group_id)),
                    ])
                ]
            )
        });

        let printed = print(&allocator, &content);

        assert_eq!(
            printed.as_str(),
            "The referenced group breaks.\nThis group breaks because:\nIt measures with the 'if_group_breaks' variant because the referenced group breaks and that's just way too much text."
        );
    }

    /// Group IDs should work correctly even when defined out of order.
    #[test]
    fn test_out_of_order_group_ids() {
        let allocator = Allocator::default();
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };

        let content = format_with(|f| {
            let id_1 = f.group_id();
            let id_2 = f.group_id();

            write!(
                f,
                [
                    group(&token("Group with id-2")).with_id(Some(id_2)),
                    hard_line_break()
                ]
            )?;

            write!(
                f,
                [
                    group(&token("Group with id-1 does not fit on the line because it exceeds the line width of 80 characters by")).with_id(Some(id_1)),
                    hard_line_break()
                ]
            )?;

            write!(
                f,
                [
                    if_group_fits_on_line(&token("Group 2 fits")).with_group_id(Some(id_2)),
                    hard_line_break(),
                    if_group_breaks(&token("Group 1 breaks")).with_group_id(Some(id_1))
                ]
            )
        });

        let printed = print_with_options(&allocator, &content, options);
        assert_eq!(
            printed.as_str(),
            "Group with id-2
Group with id-1 does not fit on the line because it exceeds the line width of 80 characters by
Group 2 fits
Group 1 breaks"
        );
    }

    struct FormatArrayNodes<'a> {
        items: Vec<&'a dyn for<'fmt> Format<'fmt, SimpleFormatContext>>,
    }

    impl<'a> Format<'a, SimpleFormatContext> for FormatArrayNodes<'_> {
        fn format(&self, f: &mut Formatter<'_, 'a, SimpleFormatContext>) -> FormatResult<()> {
            write!(
                f,
                [group(&format_args!(
                    token("["),
                    soft_block_indent(&format_args!(
                        format_with(|f| f
                            .join_with(format_args!(token(","), soft_line_break_or_space()))
                            .entries(&self.items)
                            .finish()),
                        if_group_breaks(&token(",")),
                    )),
                    token("]")
                ))]
            )
        }
    }
}
