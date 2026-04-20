use crate::format::sizing::CharWidth;
use crate::print::{PrintOptions, Printed};

use destack_source::{File, Span};

use super::bomb::DebugDropBomb;
use crate::format::{
    ActualStart, BestFittingMode, BestFittingVariants, Condition, DedentMode, Document, FileMarker,
    FormatNode, FormatTag, FormatTagKind, GroupId, GroupMode, IndentStyle, Indentation,
    InvalidDocumentError, LineMode, PrintError, PrintMode, PrintResult, TextWidth, VerbatimKind,
    tag,
};
use crate::print::call::{CallStack, FitsCallStack, PrintCallStack, PrintNodeArgs, StackFrame};
use crate::print::line::{LineSuffixEntry, LineSuffixes};
use crate::print::mode::MeasureMode;
use crate::print::queue::{
    AllPredicate, FitsEndPredicate, FitsQueue, PrintQueue, Queue, SingleEntryPredicate,
};

/// Prints the format nodes into a string
#[derive(Debug)]
pub struct Printer<'a> {
    options: PrintOptions,
    source: &'a File,
    state: PrinterState<'a>,
}

impl<'a> Printer<'a> {
    pub fn new(source: &'a File, options: PrintOptions) -> Self {
        Self {
            source,
            options,
            // NOTE #Performance: calibrate initial capacity for PrinterState
            state: PrinterState::with_capacity(source.len as usize),
        }
    }

    /// Prints the passed in node as well as all its content
    pub fn print(self, document: &'a Document) -> PrintResult<Printed> {
        self.print_with_indent(document, 0)
    }

    /// Prints the passed in node as well as all its content,
    /// starting at the specified indentation level
    pub fn print_with_indent(
        mut self,
        document: &'a Document,
        indent: u16,
    ) -> PrintResult<Printed> {
        let indentation = Indentation::Level(indent);
        self.state.pending_indent = indentation;

        let mut stack = PrintCallStack::new(PrintNodeArgs::new(indentation));
        let mut queue: PrintQueue<'a> = PrintQueue::new(document.as_ref());

        loop {
            if let Some(node) = queue.pop() {
                self.print_node(&mut stack, &mut queue, node)?;
            } else if !self.flush_line_suffixes(&mut queue, &mut stack, None) {
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

    /// Prints a single node and push the following nodes to queue
    fn print_node(
        &mut self,
        stack: &mut PrintCallStack,
        queue: &mut PrintQueue<'a>,
        node: &'a FormatNode,
    ) -> PrintResult<()> {
        #[allow(clippy::enum_glob_use)]
        use FormatTag::*;

        let args = stack.top();

        match node {
            FormatNode::Space => self.print_text(Text::Token(" ")),
            FormatNode::Token { text } => self.print_text(Text::Token(text)),
            FormatNode::Text {
                text,
                width: text_width,
            } => self.print_text(Text::Text {
                text,
                text_width: *text_width,
            }),
            FormatNode::SourcePosition { source } => {
                self.state.pending_source_position = Some(*source);
            }
            FormatNode::FileSlice {
                slice,
                width: text_width,
            } => {
                self.state.pending_source_position = Some(slice.start);
                let text = self.source.get_span_str(*slice).unwrap_or_default();
                self.print_text(Text::Text {
                    text,
                    text_width: *text_width,
                });
                self.state.pending_source_position = Some(slice.end);
            }
            FormatNode::Line(line_mode) => {
                if args.mode().is_flat()
                    && matches!(line_mode, LineMode::Soft | LineMode::SoftOrSpace)
                {
                    if line_mode == &LineMode::SoftOrSpace {
                        self.print_text(Text::Token(" "));
                    }
                } else if self.state.line_suffixes.has_pending() {
                    self.flush_line_suffixes(queue, stack, Some(node));
                } else {
                    // only print a newline if the current line isn't already empty
                    if self.state.buffer.len() > self.state.line_start {
                        self.push_marker();
                        self.print_char('\n');
                    }

                    // print a second line break if this is an empty line
                    if line_mode == &LineMode::Empty {
                        self.push_marker();
                        self.print_char('\n');
                    }

                    self.state.pending_indent = args.indentation();
                }
            }

            FormatNode::ExpandParent => {
                // handled in `Document::propagate_expands()
            }

            FormatNode::LineSuffixBoundary => {
                const HARD_BREAK: &FormatNode = &FormatNode::Line(LineMode::Hard);
                self.flush_line_suffixes(queue, stack, Some(HARD_BREAK));
            }

            FormatNode::BestFitting { variants, mode } => {
                self.print_best_fitting(variants, *mode, queue, stack)?;
            }

            FormatNode::Interned(content) => {
                queue.extend_back(content);
            }

            FormatNode::Tag(StartGroup(group)) => {
                let print_mode = match group.mode() {
                    GroupMode::Expand | GroupMode::Propagated => PrintMode::Expanded,
                    GroupMode::Flat => self.flat_group_print_mode(
                        FormatTagKind::Group,
                        group.id(),
                        args,
                        queue,
                        stack,
                    )?,
                };

                if let Some(id) = group.id() {
                    self.state.group_modes.insert_print_mode(id, print_mode);
                }

                stack.push(FormatTagKind::Group, args.with_print_mode(print_mode));
            }

            FormatNode::Tag(StartBestFitParenthesize { id }) => {
                const OPEN_PARENTHESIS: FormatNode = FormatNode::Token { text: "(" };
                const INDENT: FormatNode = FormatNode::Tag(FormatTag::StartIndent);
                const HARD_LINE_BREAK: FormatNode = FormatNode::Line(LineMode::Hard);

                let fits_flat = self.flat_group_print_mode(
                    FormatTagKind::BestFitParenthesize,
                    *id,
                    args,
                    queue,
                    stack,
                )? == PrintMode::Flat;

                let print_mode = if fits_flat {
                    PrintMode::Flat
                } else {
                    // test if the content fits in expanded mode. If not, prefer avoiding the parentheses
                    // over parenthesizing the expression.
                    if let Some(id) = id {
                        self.state
                            .group_modes
                            .insert_print_mode(*id, PrintMode::Expanded);
                    }

                    stack.push(
                        FormatTagKind::BestFitParenthesize,
                        args.with_measure_mode(MeasureMode::AllLines),
                    );

                    queue.extend_back(&[OPEN_PARENTHESIS, INDENT, HARD_LINE_BREAK]);
                    let fits_expanded = self.fits(queue, stack)?;
                    queue.pop_slice();
                    stack.pop(FormatTagKind::BestFitParenthesize)?;

                    if fits_expanded {
                        PrintMode::Expanded
                    } else {
                        PrintMode::Flat
                    }
                };

                if let Some(id) = id {
                    self.state.group_modes.insert_print_mode(*id, print_mode);
                }

                if print_mode.is_expanded() {
                    // parenthesize the content. The `EndIndent` is handled inside of the `EndBestFitParenthesize`
                    queue.extend_back(&[OPEN_PARENTHESIS, INDENT, HARD_LINE_BREAK]);
                }

                stack.push(
                    FormatTagKind::BestFitParenthesize,
                    args.with_print_mode(print_mode),
                );
            }

            FormatNode::Tag(EndBestFitParenthesize) => {
                if args.mode().is_expanded() {
                    const HARD_LINE_BREAK: FormatNode = FormatNode::Line(LineMode::Hard);
                    const CLOSE_PAREN: FormatNode = FormatNode::Token { text: ")" };

                    // finish the indent and print the hardline break and closing parentheses.
                    stack.pop(FormatTagKind::Indent)?;
                    queue.extend_back(&[HARD_LINE_BREAK, CLOSE_PAREN]);
                }

                stack.pop(FormatTagKind::BestFitParenthesize)?;
            }

            FormatNode::Tag(StartConditionalGroup(group)) => {
                let condition = group.condition();
                let expected_mode = match condition.group_id {
                    None => args.mode(),
                    Some(id) => self.state.group_modes.get_print_mode(id)?,
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

            FormatNode::Tag(StartFill) => {
                self.print_fill_entries(queue, stack)?;
            }

            FormatNode::Tag(StartIndent) => {
                stack.push(
                    FormatTagKind::Indent,
                    args.increment_indent_level(self.options.indent_style),
                );
            }

            FormatNode::Tag(StartDedent(mode)) => {
                let args = match mode {
                    DedentMode::Level => args.decrement_indent(),
                    DedentMode::Root => args.reset_indent(),
                };
                stack.push(FormatTagKind::Dedent, args);
            }

            FormatNode::Tag(StartAlign(align)) => {
                stack.push(FormatTagKind::Align, args.set_indent_align(*align));
            }

            FormatNode::Tag(StartConditionalContent(Condition { mode, group_id })) => {
                let group_mode = match group_id {
                    None => args.mode(),
                    Some(id) => self.state.group_modes.get_print_mode(*id)?,
                };

                if *mode == group_mode {
                    stack.push(FormatTagKind::ConditionalContent, args);
                } else {
                    queue.skip_content(FormatTagKind::ConditionalContent);
                }
            }

            FormatNode::Tag(StartIndentIfGroupBreaks(group_id)) => {
                let group_mode = self.state.group_modes.get_print_mode(*group_id)?;

                let args = match group_mode {
                    PrintMode::Flat => args,
                    PrintMode::Expanded => args.increment_indent_level(self.options.indent_style),
                };

                stack.push(FormatTagKind::IndentIfGroupBreaks, args);
            }

            FormatNode::Tag(StartLineSuffix) => {
                self.state
                    .line_suffixes
                    .extend(args, queue.iter_content(FormatTagKind::LineSuffix));
            }

            FormatNode::Tag(StartVerbatim(kind)) => {
                if let VerbatimKind::Verbatim { length } = kind {
                    #[expect(clippy::cast_possible_truncation)]
                    self.state.verbatim_markers.push(Span::at(
                        self.source.id,
                        self.state.buffer.len() as u32,
                        *length,
                    ));
                }

                stack.push(FormatTagKind::Verbatim, args);
            }

            FormatNode::Tag(StartFitsExpanded(tag::FitsExpanded { condition, .. })) => {
                let condition_met = match condition {
                    Some(condition) => {
                        let group_mode = match condition.group_id {
                            Some(group_id) => self.state.group_modes.get_print_mode(group_id)?,
                            None => args.mode(),
                        };

                        condition.mode == group_mode
                    }
                    None => true,
                };

                if condition_met {
                    // we measured the inner groups all in expanded. It now is necessary to measure if the inner groups fit as well.
                    self.state.measured_group_fits = false;
                }

                stack.push(FormatTagKind::FitsExpanded, args);
            }

            FormatNode::Tag(tag @ StartEntry) => {
                stack.push(tag.kind(), args);
            }

            FormatNode::Tag(
                tag @ (EndEntry
                | EndGroup
                | EndConditionalGroup
                | EndIndent
                | EndDedent
                | EndAlign
                | EndConditionalContent
                | EndIndentIfGroupBreaks
                | EndFitsExpanded
                | EndVerbatim
                | EndLineSuffix
                | EndFill),
            ) => {
                stack.pop(tag.kind())?;
            }
        }

        Ok(())
    }

    fn fits(&mut self, queue: &PrintQueue<'a>, stack: &PrintCallStack) -> PrintResult<bool> {
        let mut measure = FitsMeasurer::new(queue, stack, self);
        let result = measure.fits(&mut AllPredicate);
        measure.finish();
        result
    }

    fn flat_group_print_mode(
        &mut self,
        kind: FormatTagKind,
        id: Option<GroupId>,
        args: PrintNodeArgs,
        queue: &PrintQueue<'a>,
        stack: &mut PrintCallStack,
    ) -> PrintResult<PrintMode> {
        let print_mode = match args.mode() {
            PrintMode::Flat if self.state.measured_group_fits => {
                // a parent group has already verified that this group fits on a single line
                // thus, just continue in flat mode
                PrintMode::Flat
            }
            // the printer is either in expanded mode or it's necessary to re-measure if the group fits
            // because the printer printed a line break
            _ => {
                self.state.measured_group_fits = true;

                if let Some(id) = id {
                    self.state
                        .group_modes
                        .insert_print_mode(id, PrintMode::Flat);
                }

                // measure to see if the group fits up on a single line. If that's the case,
                // print the group in "flat" mode, otherwise continue in expanded mode
                stack.push(kind, args.with_print_mode(PrintMode::Flat));
                let fits = self.fits(queue, stack)?;
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

    fn print_text(&mut self, text: Text<'_>) {
        self.write_pending_indent();

        self.push_marker();

        match text {
            #[expect(clippy::cast_possible_truncation)]
            Text::Token(token) => {
                self.state.buffer.push_str(token);
                self.state.line_width += token.len() as u32;
            }
            Text::Text {
                text,
                text_width: width,
            } => {
                if let Some(width) = width.width() {
                    self.state.buffer.push_str(text);
                    self.state.line_width += width.value();
                } else {
                    self.print_multiline_text(text);
                }
            }
        }
    }

    /// write pending indentation into the output buffer
    fn write_pending_indent(&mut self) {
        let indent = std::mem::take(&mut self.state.pending_indent);
        if indent.is_empty() {
            return;
        }

        let level = indent.level() as usize;
        let align = indent.align() as usize;
        let indent_width = self.options.indent_width as usize;

        // indentation write: avoid per-character print_char calls
        match self.options.indent_style {
            IndentStyle::Space => {
                let total_spaces = level.saturating_mul(indent_width).saturating_add(align);
                self.state.buffer.reserve(total_spaces);
                push_spaces(&mut self.state.buffer, total_spaces);
                self.state.line_width += total_spaces as u32;
            }
            IndentStyle::Tab => {
                self.state.buffer.reserve(level.saturating_add(align));
                push_tabs(&mut self.state.buffer, level);
                push_spaces(&mut self.state.buffer, align);
                self.state.line_width +=
                    (level.saturating_mul(indent_width).saturating_add(align)) as u32;
            }
        }
    }

    /// print a multiline text payload
    fn print_multiline_text(&mut self, text: &str) {
        // ascii fast path: stream chunks between newline and tab characters
        if text.is_ascii() {
            self.print_multiline_ascii_text(text);
            return;
        }

        for char in text.chars() {
            self.print_char(char);
        }
    }

    /// print an ascii multiline text payload
    fn print_multiline_ascii_text(&mut self, text: &str) {
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
                self.state.buffer.push_str(chunk);
                self.state.line_width += (index - run_start) as u32;
            }

            if byte == b'\n' {
                self.print_newline();
            } else {
                self.state.buffer.push('\t');
                self.state.line_width += u32::from(self.options.indent_width);
            }

            index += 1;
            run_start = index;
        }

        if run_start < bytes.len() {
            let chunk = &text[run_start..];
            self.state.buffer.push_str(chunk);
            self.state.line_width += (bytes.len() - run_start) as u32;
        }
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

    fn flush_line_suffixes(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        line_break: Option<&'a FormatNode>,
    ) -> bool {
        let suffixes = self.state.line_suffixes.take_pending();

        if suffixes.len() > 0 {
            // print this line break node again once all the line suffixes have been flushed
            if let Some(line_break) = line_break {
                queue.push(line_break);
            }

            for entry in suffixes.rev() {
                match entry {
                    LineSuffixEntry::Suffix(suffix) => {
                        queue.push(suffix);
                    }
                    LineSuffixEntry::Args(args) => {
                        const LINE_SUFFIX_END: &FormatNode =
                            &FormatNode::Tag(FormatTag::EndLineSuffix);

                        stack.push(FormatTagKind::LineSuffix, args);

                        queue.push(LINE_SUFFIX_END);
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
        variants: &'a BestFittingVariants,
        mode: BestFittingMode,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
    ) -> PrintResult<()> {
        let args = stack.top();

        if args.mode().is_flat() && self.state.measured_group_fits {
            queue.extend_back(variants.most_flat());
            self.print_entry(queue, stack, args, FormatTagKind::Entry)
        } else {
            self.state.measured_group_fits = true;
            let mut variants_iter = variants.into_iter();
            let mut current = variants_iter.next().unwrap();

            for next in variants_iter {
                // test if this variant fits and if so, use it. Otherwise try the next
                // variant.

                // try to fit only the first variant on a single line
                if !matches!(
                    current.first(),
                    Some(&FormatNode::Tag(FormatTag::StartEntry))
                ) {
                    return invalid_start_tag(FormatTagKind::Entry, current.first());
                }

                // skip the first node because we want to override the args for the entry and the
                // args must be popped from the stack as soon as it sees the matching end entry.
                let content = &current[1..];

                let entry_args = args
                    .with_print_mode(PrintMode::Flat)
                    .with_measure_mode(MeasureMode::from(mode));

                queue.extend_back(content);
                stack.push(FormatTagKind::Entry, entry_args);
                let variant_fits = self.fits(queue, stack)?;
                stack.pop(FormatTagKind::Entry)?;

                // remove the content slice because printing needs the variant WITH the start entry
                let popped_slice = queue.pop_slice();
                debug_assert_eq!(popped_slice, Some(content));

                if variant_fits {
                    queue.extend_back(current);
                    return self.print_entry(
                        queue,
                        stack,
                        args.with_print_mode(PrintMode::Flat),
                        FormatTagKind::Entry,
                    );
                }

                current = next;
            }

            // at this stage current is the most expanded.

            // no variant fits, take the last (most expanded) as fallback
            queue.extend_back(current);
            self.print_entry(
                queue,
                stack,
                args.with_print_mode(PrintMode::Expanded),
                FormatTagKind::Entry,
            )
        }
    }

    /// Tries to fit as much content as possible on a single line.
    ///
    /// `Fill` is a sequence of *item*, *separator*, *item*, *separator*, *item*, ... entries.
    /// The goal is to fit as many items (with their separators) on a single line as possible and
    /// first expand the *separator* if the content exceeds the print width and only fallback to expanding
    /// the *item*s if the *item* or the *item* and the expanded *separator* don't fit on the line.
    ///
    /// The implementation handles the following 5 cases:
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
    ) -> PrintResult<()> {
        let args = stack.top();

        // it's already known that the content fit, print all items in flat mode.
        if self.state.measured_group_fits && args.mode().is_flat() {
            stack.push(FormatTagKind::Fill, args.with_print_mode(PrintMode::Flat));
            return Ok(());
        }

        stack.push(FormatTagKind::Fill, args);

        while matches!(queue.top(), Some(FormatNode::Tag(FormatTag::StartEntry))) {
            let mut measurer = FitsMeasurer::new_flat(queue, stack, self);

            // the number of item/separator pairs that fit on the same line.
            let mut flat_pairs = 0usize;
            let mut item_fits = measurer.fill_item_fits()?;

            let last_pair_layout = if item_fits {
                // measure the remaining pairs until the first item or separator that does not fit (or the end of the fill node).
                // optimisation to avoid re-measuring the next-item twice:
                // * once when measuring if the *item*, *separator*, *next-item* fit
                // * a second time when measuring if *next-item*, *separator*, *next-next-item* fit.
                loop {
                    // item that fits without a following separator.
                    if !matches!(
                        measurer.queue.top(),
                        Some(FormatNode::Tag(FormatTag::StartEntry))
                    ) {
                        break FillPairLayout::Flat;
                    }

                    let separator_fits = measurer.fill_separator_fits(PrintMode::Flat)?;

                    // item fits but the flat separator does not.
                    if !separator_fits {
                        break FillPairLayout::ItemMaybeFlat;
                    }

                    // last item/separator pair that both fit
                    if !matches!(
                        measurer.queue.top(),
                        Some(FormatNode::Tag(FormatTag::StartEntry))
                    ) {
                        break FillPairLayout::Flat;
                    }

                    item_fits = measurer.fill_item_fits()?;

                    if item_fits {
                        flat_pairs += 1;
                    } else {
                        // item and separator both fit, but the next node doesn't.
                        // print the separator in expanded mode and then re-measure if the item now
                        // fits in the next iteration of the outer loop.
                        break FillPairLayout::ItemFlatSeparatorExpanded;
                    }
                }
            } else {
                // neither item nor separator fit, print both in expanded mode.
                FillPairLayout::Expanded
            };

            measurer.finish();

            self.state.measured_group_fits = true;

            // print all pairs that fit in flat mode.
            for _ in 0..flat_pairs {
                self.print_fill_item(queue, stack, args.with_print_mode(PrintMode::Flat))?;
                self.print_fill_separator(queue, stack, args.with_print_mode(PrintMode::Flat))?;
            }

            let item_mode = match last_pair_layout {
                FillPairLayout::Flat | FillPairLayout::ItemFlatSeparatorExpanded => PrintMode::Flat,
                FillPairLayout::Expanded => PrintMode::Expanded,
                FillPairLayout::ItemMaybeFlat => {
                    let mut measurer = FitsMeasurer::new_flat(queue, stack, self);
                    // re-measuring is required to get the measurer in the correct state for measuring the separator.
                    assert!(measurer.fill_item_fits()?);
                    let separator_fits = measurer.fill_separator_fits(PrintMode::Expanded)?;
                    measurer.finish();

                    if separator_fits {
                        PrintMode::Flat
                    } else {
                        PrintMode::Expanded
                    }
                }
            };

            self.print_fill_item(queue, stack, args.with_print_mode(item_mode))?;

            if matches!(queue.top(), Some(FormatNode::Tag(FormatTag::StartEntry))) {
                let separator_mode = match last_pair_layout {
                    FillPairLayout::Flat => PrintMode::Flat,
                    FillPairLayout::ItemFlatSeparatorExpanded
                    | FillPairLayout::Expanded
                    | FillPairLayout::ItemMaybeFlat => PrintMode::Expanded,
                };

                // push a new stack frame with print mode `Flat` for the case where the separator gets printed in expanded mode
                // but does contain a group to ensure that the group will measure "fits" with the "flat" versions of the next item/separator.
                stack.push(FormatTagKind::Fill, args.with_print_mode(PrintMode::Flat));
                self.print_fill_separator(queue, stack, args.with_print_mode(separator_mode))?;
                stack.pop(FormatTagKind::Fill)?;
            }
        }

        if queue.top() == Some(&FormatNode::Tag(FormatTag::EndFill)) {
            Ok(())
        } else {
            invalid_end_tag(FormatTagKind::Fill, stack.top_kind())
        }
    }

    /// Semantic alias for [`Self::print_entry`] for fill items.
    fn print_fill_item(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        args: PrintNodeArgs,
    ) -> PrintResult<()> {
        self.print_entry(queue, stack, args, FormatTagKind::Entry)
    }

    /// Semantic alias for [`Self::print_entry`] for fill separators.
    fn print_fill_separator(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        args: PrintNodeArgs,
    ) -> PrintResult<()> {
        self.print_entry(queue, stack, args, FormatTagKind::Entry)
    }

    /// Fully print an node (print the node itself and all its descendants)
    ///
    /// Unlike [`print_node`], this function ensures the entire node has
    /// been printed when it returns and the queue is back to its original state
    fn print_entry(
        &mut self,
        queue: &mut PrintQueue<'a>,
        stack: &mut PrintCallStack,
        args: PrintNodeArgs,
        kind: FormatTagKind,
    ) -> PrintResult<()> {
        let start_entry = queue.top();

        if queue
            .pop()
            .is_some_and(|start| start.tag_kind() == Some(kind))
        {
            stack.push(kind, args);
        } else {
            return invalid_start_tag(kind, start_entry);
        }

        let mut depth = 1u32;

        while let Some(node) = queue.pop() {
            match node {
                FormatNode::Tag(FormatTag::StartEntry) => {
                    depth += 1;
                }
                FormatNode::Tag(end_tag @ FormatTag::EndEntry) => {
                    depth -= 1;
                    // reached the end entry, pop the entry from the stack and return.
                    if depth == 0 {
                        stack.pop(end_tag.kind())?;
                        return Ok(());
                    }
                }
                _ => {
                    // fall through
                }
            }

            self.print_node(stack, queue, node)?;
        }

        invalid_end_tag(kind, stack.top_kind())
    }

    fn print_char(&mut self, char: char) {
        if char == '\n' {
            self.print_newline();
        } else {
            self.state.buffer.push(char);

            let char_width = if char.is_ascii() {
                if char == '\t' {
                    self.options.indent_width as u32
                } else {
                    1
                }
            } else {
                char.width() as u32
            };

            self.state.line_width += char_width;
        }
    }

    /// trim trailing spaces and tabs for the current line before writing a newline
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

    /// print a newline with the configured line ending
    fn print_newline(&mut self) {
        if self.options.trim_trailing_whitespace {
            self.trim_trailing_line_whitespace();
        }

        self.state
            .buffer
            .push_str(self.options.line_ending.as_str());

        self.state.line_width = 0;
        self.state.line_start = self.state.buffer.len();

        // fit's only tests if groups up to the first line break fit.
        // the next group must re-measure if it still fits.
        self.state.measured_group_fits = false;
    }
}

#[derive(Copy, Clone, Debug)]
enum FillPairLayout {
    /// The item, separator, and next item fit. Print the first item and the separator in flat mode.
    Flat,
    /// The item and separator fit but the next node does not. Print the item in flat mode and
    /// the separator in expanded mode.
    ItemFlatSeparatorExpanded,
    /// The item does not fit. Print the item and any potential separator in expanded mode.
    Expanded,
    /// The item fits but the separator does not in flat mode. If the separator fits in expanded mode then
    /// print the item in flat and the separator in expanded mode, otherwise print both in expanded mode.
    ItemMaybeFlat,
}

/// Printer state that is global to all nodes.
/// Stores the result of the print operation (buffer and mappings) and at what
/// position the printer currently is.
#[derive(Default, Debug)]
struct PrinterState<'a> {
    /// The formatted output.
    buffer: String,

    /// The source markers that map source positions to formatted positions.
    source_markers: Vec<FileMarker>,

    /// The next source position that should be flushed when writing the next text.
    pending_source_position: Option<u32>,

    /// The current indentation that should be written before the next text.
    pending_indent: Indentation,

    /// Caches if the code up to the next newline has been measured to fit on a single line.
    /// This is used to avoid re-measuring the same content multiple times.
    measured_group_fits: bool,

    /// The offset at which the current line in `buffer` starts.
    line_start: usize,

    /// The accumulated unicode-width of all characters on the current line.
    line_width: u32,

    /// The line suffixes that should be printed at the end of the line.
    line_suffixes: LineSuffixes<'a>,
    verbatim_markers: Vec<Span>,
    group_modes: GroupModes,
    // Reused queue to measure if a group fits. Optimisation to avoid re-allocating a new
    // vec every time a group gets measured
    fits_stack: Vec<StackFrame>,
    fits_queue: Vec<&'a [FormatNode]>,
}

impl PrinterState<'_> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            ..Self::default()
        }
    }
}

/// Tracks the mode in which groups with ids are printed. Stores the groups at `group.id()` index.
/// This is based on the assumption that the group ids for a single document are dense.
#[derive(Debug, Default)]
struct GroupModes(Vec<Option<PrintMode>>);

impl GroupModes {
    fn insert_print_mode(&mut self, group_id: GroupId, mode: PrintMode) {
        let index = u32::from(group_id) as usize;

        if self.0.len() <= index {
            self.0.resize(index + 1, None);
        }

        self.0[index] = Some(mode);
    }

    fn get_print_mode(&self, group_id: GroupId) -> PrintResult<PrintMode> {
        let index = u32::from(group_id) as usize;

        match self.0.get(index) {
            Some(Some(print_mode)) => Ok(*print_mode),
            None | Some(None) => Err(PrintError::InvalidDocument(
                InvalidDocumentError::UnknownGroupId { group_id },
            )),
        }
    }
}

#[must_use = "FitsMeasurer must be finished."]
struct FitsMeasurer<'a, 'print> {
    state: FitsState,
    queue: FitsQueue<'a, 'print>,
    stack: FitsCallStack<'print>,
    printer: &'print mut Printer<'a>,
    must_be_flat: bool,

    /// Bomb that enforces that finish is explicitly called to restore the `fits_stack` and `fits_queue` vectors.
    bomb: DebugDropBomb,
}

impl<'a, 'print> FitsMeasurer<'a, 'print> {
    fn new_flat(
        print_queue: &'print PrintQueue<'a>,
        print_stack: &'print PrintCallStack,
        printer: &'print mut Printer<'a>,
    ) -> Self {
        let mut measurer = Self::new(print_queue, print_stack, printer);
        measurer.must_be_flat = true;
        measurer
    }

    fn new(
        print_queue: &'print PrintQueue<'a>,
        print_stack: &'print PrintCallStack,
        printer: &'print mut Printer<'a>,
    ) -> Self {
        let saved_stack = std::mem::take(&mut printer.state.fits_stack);
        let saved_queue = std::mem::take(&mut printer.state.fits_queue);
        debug_assert!(saved_stack.is_empty());
        debug_assert!(saved_queue.is_empty());

        let fits_queue = FitsQueue::new(print_queue, saved_queue);
        let fits_stack = FitsCallStack::new(print_stack, saved_stack);

        let fits_state = FitsState {
            pending_indent: printer.state.pending_indent,
            line_width: printer.state.line_width,
            has_line_suffix: printer.state.line_suffixes.has_pending(),
        };

        Self {
            state: fits_state,
            queue: fits_queue,
            stack: fits_stack,
            must_be_flat: false,
            printer,
            bomb: DebugDropBomb::new(
                "MeasurerFits must be `finished` to restore the `fits_queue` and `fits_stack`.",
            ),
        }
    }

    /// Tests if it's possible to print the content of the queue up to the first hard line break
    /// or the end of the document on a single line without exceeding the line width.
    fn fits<P>(&mut self, predicate: &mut P) -> PrintResult<bool>
    where
        P: FitsEndPredicate,
    {
        while let Some(node) = self.queue.pop() {
            match self.fits_node(node)? {
                Fits::Yes => return Ok(true),
                Fits::No => {
                    return Ok(false);
                }
                Fits::Maybe => {
                    if predicate.is_end(node)? {
                        break;
                    }

                    continue;
                }
            }
        }

        Ok(true)
    }

    /// Tests if the content of a `Fill` item fits in [`PrintMode::Flat`].
    ///
    /// Returns `Err` if the top node of the queue is not a [`Tag::StartEntry`]
    /// or if the document has any mismatching start/end tags.
    fn fill_item_fits(&mut self) -> PrintResult<bool> {
        self.fill_entry_fits(PrintMode::Flat)
    }

    /// Tests if the content of a `Fill` separator fits with `mode`.
    ///
    /// Returns `Err` if the top node of the queue is not a [`Tag::StartEntry`]
    /// or if the document has any mismatching start/end tags.
    fn fill_separator_fits(&mut self, mode: PrintMode) -> PrintResult<bool> {
        self.fill_entry_fits(mode)
    }

    /// Tests if the nodes between the [`Tag::StartEntry`] and [`Tag::EndEntry`]
    /// of a fill item or separator fits with `mode`.
    ///
    /// Returns `Err` if the queue isn't positioned at a [`Tag::StartEntry`] or if
    /// the matching [`Tag::EndEntry`] is missing.
    fn fill_entry_fits(&mut self, mode: PrintMode) -> PrintResult<bool> {
        let start_entry = self.queue.top();

        if !matches!(start_entry, Some(&FormatNode::Tag(FormatTag::StartEntry))) {
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

    /// Tests if the passed node fits on the current line or not.
    fn fits_node(&mut self, node: &'a FormatNode) -> PrintResult<Fits> {
        #[allow(clippy::enum_glob_use)]
        use FormatTag::*;

        let args = self.stack.top();

        match node {
            FormatNode::Space => return Ok(self.fits_text(Text::Token(" "), args)),

            FormatNode::Line(line_mode) => {
                match args.mode() {
                    PrintMode::Flat => match line_mode {
                        LineMode::SoftOrSpace => return Ok(self.fits_text(Text::Token(" "), args)),
                        LineMode::Soft => {}
                        LineMode::Hard | LineMode::Empty => {
                            return Ok(if self.must_be_flat {
                                Fits::No
                            } else {
                                Fits::Yes
                            });
                        }
                    },
                    PrintMode::Expanded => {
                        match args.measure_mode() {
                            MeasureMode::FirstLine => {
                                // Reachable if the restQueue contains an node with mode expanded because Expanded
                                // is what the mode's initialized to by default
                                // This means, the printer is outside of the current node at this point and any
                                // line break should be printed as regular line break
                                return Ok(Fits::Yes);
                            }
                            MeasureMode::AllLines | MeasureMode::AllLinesAllowTextOverflow => {
                                // Continue measuring on the next line
                                self.state.line_width = 0;
                                self.state.pending_indent = args.indentation();
                            }
                        }
                    }
                }
            }

            FormatNode::Token { text } => return Ok(self.fits_text(Text::Token(text), args)),
            FormatNode::Text {
                text,
                width: text_width,
            } => {
                return Ok(self.fits_text(
                    Text::Text {
                        text,
                        text_width: *text_width,
                    },
                    args,
                ));
            }
            FormatNode::SourcePosition { .. } => {}
            FormatNode::FileSlice {
                slice,
                width: text_width,
            } => {
                let text = self.printer.source.get_span_str(*slice).unwrap_or_default();
                return Ok(self.fits_text(
                    Text::Text {
                        text,
                        text_width: *text_width,
                    },
                    args,
                ));
            }
            FormatNode::LineSuffixBoundary => {
                if self.state.has_line_suffix {
                    return Ok(Fits::No);
                }
            }

            FormatNode::ExpandParent => {
                if self.must_be_flat {
                    return Ok(Fits::No);
                }
            }

            FormatNode::BestFitting { variants, mode } => {
                let slice = match args.mode() {
                    PrintMode::Flat => (
                        variants.most_flat(),
                        args.with_measure_mode(MeasureMode::from(*mode)),
                    ),
                    PrintMode::Expanded => (variants.most_expanded(), args),
                }
                .0;

                if !matches!(slice.first(), Some(FormatNode::Tag(FormatTag::StartEntry))) {
                    return invalid_start_tag(FormatTagKind::Entry, slice.first());
                }

                self.queue.extend_back(slice);
            }

            FormatNode::Interned(content) => self.queue.extend_back(content),

            FormatNode::Tag(StartIndent) => {
                self.stack.push(
                    FormatTagKind::Indent,
                    args.increment_indent_level(self.options().indent_style),
                );
            }

            FormatNode::Tag(StartDedent(mode)) => {
                let args = match mode {
                    DedentMode::Level => args.decrement_indent(),
                    DedentMode::Root => args.reset_indent(),
                };
                self.stack.push(FormatTagKind::Dedent, args);
            }

            FormatNode::Tag(StartAlign(align)) => {
                self.stack
                    .push(FormatTagKind::Align, args.set_indent_align(*align));
            }

            FormatNode::Tag(StartGroup(group)) => {
                return Ok(self.fits_group(FormatTagKind::Group, group.mode(), group.id(), args));
            }

            FormatNode::Tag(StartBestFitParenthesize { id }) => {
                if let Some(id) = id {
                    self.printer
                        .state
                        .group_modes
                        .insert_print_mode(*id, args.mode());
                }

                // Don't use the parenthesized with indent layout even when measuring expanded mode similar to `BestFitting`.
                // This is to expand the left and not right after the `(` parentheses (it is okay to expand after the content that it wraps).
                self.stack.push(FormatTagKind::BestFitParenthesize, args);
            }

            FormatNode::Tag(EndBestFitParenthesize) => {
                // If this is the end tag of the outer most parentheses for which we measure if it fits,
                // pop the indent.
                if args.mode().is_expanded() && self.stack.top_kind() == Some(FormatTagKind::Indent)
                {
                    self.stack.pop(FormatTagKind::Indent).unwrap();
                    let unindented = self.stack.pop(FormatTagKind::BestFitParenthesize)?;

                    // There's a hard line break after the indent but don't return `Fits::Yes` here
                    // to ensure any trailing comments (that, unfortunately, are attached to the statement and not the expression)
                    // fit too.
                    self.state.line_width = 0;
                    self.state.pending_indent = unindented.indentation();

                    return Ok(self.fits_text(Text::Token(")"), unindented));
                }

                self.stack.pop(FormatTagKind::BestFitParenthesize)?;
            }

            FormatNode::Tag(StartConditionalGroup(group)) => {
                let condition = group.condition();

                let print_mode = match condition.group_id {
                    None => args.mode(),
                    Some(group_id) => self.group_modes().get_print_mode(group_id)?,
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

            FormatNode::Tag(StartConditionalContent(condition)) => {
                let print_mode = match condition.group_id {
                    None => args.mode(),
                    Some(group_id) => self.group_modes().get_print_mode(group_id)?,
                };

                if condition.mode == print_mode {
                    self.stack.push(FormatTagKind::ConditionalContent, args);
                } else {
                    self.queue.skip_content(FormatTagKind::ConditionalContent);
                }
            }

            FormatNode::Tag(StartIndentIfGroupBreaks(id)) => {
                let print_mode = self.group_modes().get_print_mode(*id)?;

                match print_mode {
                    PrintMode::Flat => {
                        self.stack.push(FormatTagKind::IndentIfGroupBreaks, args);
                    }
                    PrintMode::Expanded => {
                        self.stack.push(
                            FormatTagKind::IndentIfGroupBreaks,
                            args.increment_indent_level(self.options().indent_style),
                        );
                    }
                }
            }

            FormatNode::Tag(StartLineSuffix) => {
                self.queue.skip_content(FormatTagKind::LineSuffix);
                self.state.has_line_suffix = true;
            }

            FormatNode::Tag(EndLineSuffix) => {
                return invalid_end_tag(FormatTagKind::LineSuffix, self.stack.top_kind());
            }

            FormatNode::Tag(StartFitsExpanded(tag::FitsExpanded {
                condition,
                propagate_expand,
            })) => {
                match args.mode() {
                    PrintMode::Expanded => {
                        // As usual, nothing to measure
                        self.stack.push(FormatTagKind::FitsExpanded, args);
                    }
                    PrintMode::Flat => {
                        let condition_met = match condition {
                            Some(condition) => {
                                let group_mode = match condition.group_id {
                                    Some(group_id) => {
                                        self.group_modes().get_print_mode(group_id)?
                                    }
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
                            if propagate_expand.get() {
                                return Ok(Fits::No);
                            }

                            // As usual
                            self.stack.push(FormatTagKind::FitsExpanded, args);
                        }
                    }
                }
            }

            FormatNode::Tag(tag @ (StartFill | StartVerbatim(_) | StartEntry)) => {
                self.stack.push(tag.kind(), args);
            }

            FormatNode::Tag(
                tag @ (EndFill
                | EndVerbatim
                | EndEntry
                | EndGroup
                | EndConditionalGroup
                | EndIndentIfGroupBreaks
                | EndConditionalContent
                | EndAlign
                | EndDedent
                | EndIndent
                | EndFitsExpanded),
            ) => {
                self.stack.pop(tag.kind())?;
            }
        }

        Ok(Fits::Maybe)
    }

    fn fits_group(
        &mut self,
        kind: FormatTagKind,
        group_mode: GroupMode,
        id: Option<GroupId>,
        args: PrintNodeArgs,
    ) -> Fits {
        if self.must_be_flat && !group_mode.is_flat() {
            return Fits::No;
        }

        // Continue printing groups in expanded mode if measuring a `best_fitting` node where
        // a group expands.
        let print_mode = if group_mode.is_flat() {
            args.mode()
        } else {
            PrintMode::Expanded
        };

        self.stack.push(kind, args.with_print_mode(print_mode));

        if let Some(id) = id {
            self.group_modes_mut().insert_print_mode(id, print_mode);
        }

        Fits::Maybe
    }

    fn fits_text(&mut self, text: Text<'_>, args: PrintNodeArgs) -> Fits {
        let indent = std::mem::take(&mut self.state.pending_indent);
        if !indent.is_empty() {
            self.state.line_width += u32::from(indent.level())
                * u32::from(self.options().indent_width)
                + u32::from(indent.align());
        }

        match text {
            #[expect(clippy::cast_possible_truncation)]
            Text::Token(token) => {
                self.state.line_width += token.len() as u32;
            }
            Text::Text { text, text_width } => {
                if let Some(width) = text_width.width() {
                    self.state.line_width += width.value();
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
                            c => c.width(),
                        };
                        self.state.line_width += char_width as u32;
                    }
                }
            }
        }

        if self.exceeds_width(args) {
            return Fits::No;
        }

        Fits::Maybe
    }

    /// return true if the measured line width exceeds the line limit
    fn exceeds_width(&self, args: PrintNodeArgs) -> bool {
        self.state.line_width > self.options().line_width.into()
            && !args.measure_mode().allows_text_overflow()
    }

    /// measure ascii text with newline or tab handling without char decoding
    fn fits_multiline_ascii_text(&mut self, text: &str, args: PrintNodeArgs) -> Fits {
        let bytes = text.as_bytes();
        let mut run_start = 0usize;
        let mut index = 0usize;

        while index < bytes.len() {
            let byte = bytes[index];

            if byte != b'\n' && byte != b'\t' {
                index += 1;
                continue;
            }

            self.state.line_width += (index - run_start) as u32;

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
                self.state.line_width += u32::from(self.options().indent_width);
            }

            index += 1;
            run_start = index;
        }

        self.state.line_width += (bytes.len() - run_start) as u32;
        Fits::Maybe
    }

    fn finish(mut self) {
        self.bomb.defuse();

        let mut queue = self.queue.finish();
        queue.clear();
        self.printer.state.fits_queue = queue;

        let mut stack = self.stack.finish();
        stack.clear();
        self.printer.state.fits_stack = stack;
    }

    fn options(&self) -> &PrintOptions {
        &self.printer.options
    }

    fn group_modes(&self) -> &GroupModes {
        &self.printer.state.group_modes
    }

    fn group_modes_mut(&mut self) -> &mut GroupModes {
        &mut self.printer.state.group_modes
    }
}

#[cold]
pub(crate) fn invalid_end_tag<R>(
    end_tag: FormatTagKind,
    start_tag: Option<FormatTagKind>,
) -> PrintResult<R> {
    Err(PrintError::InvalidDocument(match start_tag {
        None => InvalidDocumentError::StartTagMissing { kind: end_tag },
        Some(kind) => InvalidDocumentError::StartEndTagMismatch {
            start_kind: end_tag,
            end_kind: kind,
        },
    }))
}

#[cold]
pub(crate) fn invalid_start_tag<R>(
    expected: FormatTagKind,
    actual: Option<&FormatNode>,
) -> PrintResult<R> {
    let start = match actual {
        None => ActualStart::EndOfDocument,
        Some(FormatNode::Tag(tag)) => {
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
    // Node fits
    Yes,
    // Node doesn't fit
    No,
    // Node may fit, depends on the nodes following it
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

/// State used when measuring if a group fits on a single line
#[derive(Debug)]
struct FitsState {
    pending_indent: Indentation,
    has_line_suffix: bool,
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
    use destack_source::{File, FileType};

    use crate::format::{Document, FormatState, IndentStyle, LineEnding, VecBuffer};
    use crate::prelude::*;
    use crate::print::{PrintOptions, Printed, Printer};
    use crate::{format_args, write};

    fn format(root: &dyn Format<SimpleFormatContext>) -> Printed {
        format_with_options(root, PrintOptions::default())
    }

    fn format_with_options(
        root: &dyn Format<SimpleFormatContext>,
        options: PrintOptions,
    ) -> Printed {
        let formatted = crate::format!(SimpleFormatContext::empty_destack(), [root]).unwrap();

        Printer::new(&File::empty_text(FileType::Destack), options)
            .print(formatted.document())
            .expect("Document to be valid")
    }

    /// Groups that fit within line width should print on one line.
    #[test]
    fn test_prints_group_on_single_line_if_fits() {
        let result = format(&FormatArrayNodes {
            items: vec![
                &token("\"a\""),
                &token("\"b\""),
                &token("\"c\""),
                &token("\"d\""),
            ],
        });

        assert_eq!(r#"["a", "b", "c", "d"]"#, result.as_str());
    }

    /// Nested indentation should accumulate correctly.
    #[test]
    fn test_tracks_indent_for_each_token() {
        let formatted = format(&format_args!(
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
        ));

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
        let options = PrintOptions {
            line_ending: LineEnding::CarriageReturnLineFeed,
            ..PrintOptions::default()
        };

        let result = format_with_options(
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
        let result = format_with_options(
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
        let result = format_with_options(
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
        let result = format_with_options(
            &format_args![text("a\t"), hard_line_break()],
            PrintOptions::default().with_trim_trailing_whitespace(true),
        );

        assert_eq!("a\n", result.as_str());
    }

    /// Groups containing strings with newlines should break.
    #[test]
    fn test_breaks_group_if_string_contains_newline() {
        let result = format(&FormatArrayNodes {
            items: vec![
                &text("`This is a string spanning\ntwo lines`"),
                &token("\"b\""),
            ],
        });

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
        let result = format(&group(&format_args![token("a"), block_indent(&token("b"))]));

        assert_eq!("a\n    b\n", result.as_str());
    }

    /// Parent groups should break when child content doesn't fit.
    #[test]
    fn test_breaks_parent_groups_if_dont_fit_on_single_line() {
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = format_with_options(
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
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = format_with_options(
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
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };
        let result = format_with_options(
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
        let options = PrintOptions {
            indent_width: 4,
            line_width: 19,
            indent_style: IndentStyle::Tab,
            ..PrintOptions::default()
        };
        let result = format_with_options(
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
        let result = format(&format_args![
            token("a"),
            hard_line_break(),
            hard_line_break(),
            hard_line_break(),
            token("b"),
        ]);

        assert_eq!("a\nb", result.as_str());
    }

    /// Empty lines should not collapse.
    #[test]
    fn test_prints_consecutive_empty_lines_as_many() {
        let result = format(&format_args![
            token("a"),
            empty_line(),
            empty_line(),
            empty_line(),
            token("b"),
        ]);

        assert_eq!("a\n\n\n\nb", result.as_str());
    }

    /// Mixed empty lines and hard breaks should preserve empty lines.
    #[test]
    fn test_prints_consecutive_mixed_lines_as_many() {
        let result = format(&format_args![
            token("a"),
            empty_line(),
            hard_line_break(),
            empty_line(),
            hard_line_break(),
            token("b"),
        ]);

        assert_eq!("a\n\n\nb", result.as_str());
    }

    /// Fill should break items optimally based on line width.
    #[test]
    fn test_fill_breaks() {
        let mut state = FormatState::new(SimpleFormatContext::empty_destack());
        let mut buffer = VecBuffer::new(&mut state);
        let mut formatter = Formatter::new(&mut buffer);

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

        let document = Document::from(buffer.into_vec());

        let printed = Printer::new(
            &File::empty_text(FileType::Destack),
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
        let printed = format(&format_args![
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
        ]);

        assert_eq!(printed.as_str(), "[1, 2, 3]; // trailing");
    }

    /// Conditional formatting should work correctly with group IDs.
    #[test]
    fn test_conditional_with_group_id_in_fits() {
        let content = format_with(|f| {
            let group_id = f.group_id("test");
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

        let printed = format(&content);

        assert_eq!(
            printed.as_str(),
            "The referenced group breaks.\nThis group breaks because:\nIt measures with the 'if_group_breaks' variant because the referenced group breaks and that's just way too much text."
        );
    }

    /// Group IDs should work correctly even when defined out of order.
    #[test]
    fn test_out_of_order_group_ids() {
        let options = PrintOptions {
            line_width: 80,
            ..PrintOptions::default()
        };

        let content = format_with(|f| {
            let id_1 = f.group_id("id-1");
            let id_2 = f.group_id("id-2");

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

        let printed = format_with_options(&content, options);
        assert_eq!(
            printed.as_str(),
            "Group with id-2
Group with id-1 does not fit on the line because it exceeds the line width of 80 characters by
Group 2 fits
Group 1 breaks"
        );
    }

    struct FormatArrayNodes<'a> {
        items: Vec<&'a dyn Format<SimpleFormatContext>>,
    }

    impl Format<SimpleFormatContext> for FormatArrayNodes<'_> {
        fn format(&self, f: &mut Formatter<'_, SimpleFormatContext>) -> FormatResult<()> {
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
