use rustc_hash::FxHashMap;

use crate::format::{
    Allocator, ArenaVec, BestFittingMode, BestFittingVariants, Format, FormatContext,
    FormatElement, FormatResult, Formatter, Instruction, InstructionIter, InstructionSlice,
    InstructionTape, LineMode, Opcode,
};

/// Content formatted without soft line behavior.
#[derive(Debug)]
pub struct WithoutSoftLines<'fmt, Content> {
    /// The content to format.
    content: &'fmt Content,
}

impl<'fmt, Content> WithoutSoftLines<'fmt, Content> {
    /// Create content formatted without soft line behavior.
    pub const fn new(content: &'fmt Content) -> Self {
        Self { content }
    }
}

impl<'a, Context, Content> Format<'a, Context> for WithoutSoftLines<'_, Content>
where
    Context: FormatContext,
    Content: Format<'a, Context>,
{
    fn format(&self, formatter: &mut Formatter<'_, 'a, Context>) -> FormatResult<()> {
        // capture content before rewriting its instruction graph
        let instructions = formatter.capture_tape(self.content)?;
        if instructions.is_empty() {
            return Ok(());
        }

        // rewrite all reachable instruction slices once
        let mut rewriter = SoftLineRewriter::new(formatter.allocator());
        let instructions = instructions.into_slice();
        let rewritten = rewriter.rewrite_slice(instructions);
        formatter.write_element(FormatElement::Slice(rewritten));

        Ok(())
    }
}

/// Create content formatted without soft line behavior.
pub const fn without_soft_lines<Content>(content: &Content) -> WithoutSoftLines<'_, Content> {
    WithoutSoftLines::new(content)
}

impl<'a> FormatElement<'a> {
    /// Remove soft line behavior from this operation and its nested instruction slices.
    pub fn remove_soft_lines(self, allocator: &'a Allocator) -> Option<Self> {
        match self {
            Self::Line(LineMode::Soft) => None,
            Self::Line(LineMode::SoftOrSpace) => Some(Self::Space),
            Self::Slice(instructions) => {
                let mut rewriter = SoftLineRewriter::new(allocator);
                let instructions = rewriter.rewrite_slice(instructions);

                if instructions.is_empty() {
                    None
                } else {
                    Some(Self::Slice(instructions))
                }
            }
            element => Some(element),
        }
    }
}

/// A graph rewriter that removes soft line behavior.
struct SoftLineRewriter<'a> {
    /// The formatter arena.
    allocator: &'a Allocator,
    /// Rewritten slices by source pointer identity.
    slices: FxHashMap<*const (), InstructionSlice<'a>>,
}

impl<'a> SoftLineRewriter<'a> {
    /// Create one rewriter in a formatter arena.
    fn new(allocator: &'a Allocator) -> Self {
        Self {
            allocator,
            slices: FxHashMap::default(),
        }
    }

    /// Rewrite one instruction slice.
    fn rewrite_slice(&mut self, instructions: InstructionSlice<'a>) -> InstructionSlice<'a> {
        let identity = instructions.as_ptr().cast();
        if let Some(rewritten) = self.slices.get(&identity) {
            return *rewritten;
        }

        // traverse nested slices explicitly from leaves to root
        let mut frames = vec![SoftLineFrame::new(instructions, self.allocator)];
        let mut completed = None;

        loop {
            // apply one completed child slice to its parent continuation
            if let Some(rewritten) = completed.take() {
                let Some(parent) = frames.last_mut() else {
                    return rewritten;
                };
                let continuation = parent.continuation.take();
                debug_assert!(continuation.is_some());
                // safety: completed children are only produced for a pending continuation
                let continuation = unsafe { continuation.unwrap_unchecked() };

                match continuation {
                    SoftLineContinuation::Slice => parent.push_slice(rewritten),
                    SoftLineContinuation::BestFitting {
                        mut remaining,
                        mut rewritten_variants,
                        mode,
                    } => {
                        rewritten_variants.push(rewritten);

                        if let Some(next) = remaining.next().copied() {
                            parent.continuation = Some(SoftLineContinuation::BestFitting {
                                remaining,
                                rewritten_variants,
                                mode,
                            });

                            if let Some(rewritten) = self.slices.get(&next.as_ptr().cast()) {
                                completed = Some(*rewritten);
                            } else {
                                frames.push(SoftLineFrame::new(next, self.allocator));
                            }
                        } else {
                            parent.push_best_fitting(rewritten_variants, mode);
                        }
                    }
                }

                continue;
            }

            // decode one instruction from the current frame
            // safety: the root frame remains until it produces the returned completion
            let frame = unsafe { frames.last_mut().unwrap_unchecked() };
            let Some(instruction) = frame.instructions.next() else {
                // safety: the active frame was read immediately above
                let frame = unsafe { frames.pop().unwrap_unchecked() };
                let identity = frame.original.as_ptr().cast();
                let rewritten = frame.finish();
                self.slices.insert(identity, rewritten);
                completed = Some(rewritten);
                continue;
            };

            if !frame.filter.retain(instruction) {
                continue;
            }

            // rewrite instructions with nested or soft-line behavior
            let nested = match instruction.opcode() {
                Opcode::SoftOrSpaceLine => {
                    frame.output.push(Opcode::Space.encode(0));
                    None
                }
                Opcode::Slice => {
                    frame.continuation = Some(SoftLineContinuation::Slice);
                    Some(instruction.slice())
                }
                Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                    let (variants, mode) = instruction.best_fitting();
                    let variants = variants.as_slice();
                    let rewritten_variants =
                        ArenaVec::with_capacity_in(variants.len(), self.allocator);

                    // safety: BestFittingVariants always contains at least two variants
                    let first = unsafe { *variants.get_unchecked(0) };
                    let remaining = unsafe { variants.get_unchecked(1..) }.iter();
                    frame.continuation = Some(SoftLineContinuation::BestFitting {
                        remaining,
                        rewritten_variants,
                        mode,
                    });
                    Some(first)
                }
                _ => {
                    frame.output.push(instruction);
                    None
                }
            };

            // process one nested slice or reuse its memoized result
            if let Some(nested) = nested {
                if let Some(rewritten) = self.slices.get(&nested.as_ptr().cast()) {
                    completed = Some(*rewritten);
                } else {
                    frames.push(SoftLineFrame::new(nested, self.allocator));
                }
            }
        }
    }
}

/// One active instruction slice rewrite.
struct SoftLineFrame<'a> {
    /// The original instruction slice.
    original: InstructionSlice<'a>,
    /// The remaining source instructions.
    instructions: InstructionIter<'a>,
    /// The rewritten instructions.
    output: InstructionTape<'a>,
    /// The conditional content state at this slice depth.
    filter: SoftLineFilter,
    /// The nested instruction awaiting a rewritten child.
    continuation: Option<SoftLineContinuation<'a>>,
}

impl<'a> SoftLineFrame<'a> {
    /// Create one rewrite frame.
    fn new(original: InstructionSlice<'a>, allocator: &'a Allocator) -> Self {
        Self {
            original,
            instructions: original.iter(),
            output: InstructionTape::with_capacity(original.len(), allocator),
            filter: SoftLineFilter::default(),
            continuation: None,
        }
    }

    /// Append one rewritten nested slice.
    fn push_slice(&mut self, instructions: InstructionSlice<'a>) {
        self.output
            .push(Opcode::Slice.encode_with(0, instructions.as_ptr() as usize as u64));
    }

    /// Append one rewritten best-fitting instruction.
    fn push_best_fitting(
        &mut self,
        variants: ArenaVec<'a, InstructionSlice<'a>>,
        mode: BestFittingMode,
    ) {
        let variants = BestFittingVariants::from_vec_unchecked(variants);
        let opcode = match mode {
            BestFittingMode::FirstLine => Opcode::BestFittingFirstLine,
            BestFittingMode::AllLines => Opcode::BestFittingAllLines,
        };

        self.output.push(opcode.encode_with(
            variants.as_slice().len() as u64,
            variants.as_slice().as_ptr() as usize as u64,
        ));
    }

    /// Store this completed frame as a stable instruction slice.
    fn finish(self) -> InstructionSlice<'a> {
        debug_assert!(self.continuation.is_none());

        self.output.into_slice()
    }
}

/// The parent operation awaiting one rewritten child slice.
enum SoftLineContinuation<'a> {
    /// One ordinary nested instruction slice.
    Slice,
    /// One sequence of best-fitting variants.
    BestFitting {
        /// The source variants after the active child.
        remaining: std::slice::Iter<'a, InstructionSlice<'a>>,
        /// The rewritten variants before the active child.
        rewritten_variants: ArenaVec<'a, InstructionSlice<'a>>,
        /// The variant measurement mode.
        mode: BestFittingMode,
    },
}

/// Conditional content state hidden by soft line removal.
#[derive(Copy, Clone, Debug, Default)]
struct SoftLineFilter {
    /// The hidden expanded conditional depth.
    hidden_depth: usize,
}

impl SoftLineFilter {
    /// Advance conditional nesting and return whether one instruction should remain.
    fn retain(&mut self, instruction: Instruction<'_>) -> bool {
        let opcode = instruction.opcode();

        // discard instructions inside expanded conditional content
        if self.hidden_depth > 0 {
            match opcode {
                Opcode::StartConditionalFlat | Opcode::StartConditionalExpanded => {
                    self.hidden_depth += 1;
                }
                Opcode::EndConditionalContent => {
                    self.hidden_depth -= 1;
                }
                _ => {}
            }

            return false;
        }

        // remove soft lines and unwrap flat conditional content
        match opcode {
            Opcode::SoftLine | Opcode::StartConditionalFlat | Opcode::EndConditionalContent => {
                false
            }
            Opcode::StartConditionalExpanded => {
                self.hidden_depth = 1;
                false
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{
        FormatElement, FormatLayout, FormatState, Formatter, LineMode, SimpleFormatContext,
        without_soft_lines,
    };
    use crate::prelude::*;
    use crate::{format, format_args};

    /// Remove soft line behavior while retaining ordinary content.
    #[test]
    fn test_remove_soft_lines() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_tspp(),
            [without_soft_lines(&format_args!(
                token("left"),
                soft_line_break_or_space(),
                token("middle"),
                soft_line_break(),
                token("right")
            ))]
        )
        .unwrap();

        assert_eq!(formatted.print().unwrap().as_str(), "left middleright");
    }

    /// Remove slices that contain only discarded soft lines.
    #[test]
    fn test_remove_soft_line_slice() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_tspp(), &allocator);
        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Line(LineMode::Soft));
        let instructions = formatter.into_tape().into_slice();

        let rewritten = FormatElement::Slice(instructions).remove_soft_lines(&allocator);

        assert_eq!(rewritten, None);
    }

    /// Discard expansion effects with removed expanded conditional content.
    #[test]
    fn test_remove_soft_lines_discards_hidden_expansion() {
        let allocator = Allocator::default();
        let hard_line = hard_line_break();
        let hidden_break = if_group_breaks(&hard_line);
        let rewritten = without_soft_lines(&hidden_break);
        let head = token("head");
        let line = soft_line_break_or_space();
        let tail = token("tail");
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_tspp(),
            [group(&format_args!(head, rewritten, line, tail))]
        )
        .unwrap();

        assert_eq!(formatted.print().unwrap().as_str(), "head tail");
    }

    /// Rewrite deeply nested instruction slices without consuming call stack.
    #[test]
    fn test_remove_soft_lines_from_deep_slices() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_tspp(), &allocator);
        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Line(LineMode::SoftOrSpace));
        let mut instructions = formatter.into_tape().into_slice();

        for _ in 0..100_000 {
            let mut formatter = Formatter::new(&mut state);
            formatter.write_element(FormatElement::Slice(instructions));
            instructions = formatter.into_tape().into_slice();
        }

        let rewritten = FormatElement::Slice(instructions)
            .remove_soft_lines(&allocator)
            .unwrap();

        assert_eq!(rewritten.single_line_width(), Some(1));
    }
}
