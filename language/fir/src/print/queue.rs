use std::fmt::Debug;
use std::iter::FusedIterator;
use std::marker::PhantomData;

use crate::format::{
    FormatTagKind, Instruction, InstructionIter, InstructionSlice, InstructionTag, Opcode,
    PrintResult,
};
use crate::print::stack::{Stack, StackedStack};
use crate::print::{invalid_start_tag, missing_end_tag};

/// One queued instruction source.
#[derive(Debug, Clone)]
pub(crate) enum QueueFrame<'a> {
    /// One encoded instruction tape.
    Tape(InstructionIter<'a>),
    /// One synthetic instruction.
    Single(Option<Instruction<'a>>),
    /// One short synthetic instruction sequence.
    Sequence {
        /// The inline instruction storage.
        instructions: [Instruction<'a>; 3],
        /// The next instruction index.
        index: u8,
        /// The populated instruction count.
        length: u8,
    },
}

impl<'a> QueueFrame<'a> {
    /// Create one frame over encoded instructions.
    fn tape(instructions: InstructionSlice<'a>) -> Self {
        Self::Tape(instructions.iter())
    }

    /// Read the next instruction.
    fn next(&mut self) -> Option<Instruction<'a>> {
        match self {
            Self::Tape(instructions) => instructions.next(),
            Self::Single(instruction) => instruction.take(),
            Self::Sequence {
                instructions,
                index,
                length,
            } => {
                if *index >= *length {
                    return None;
                }

                let instruction = instructions[usize::from(*index)];
                *index += 1;

                Some(instruction)
            }
        }
    }

    /// Return whether this frame has no remaining instructions.
    fn is_empty(&self) -> bool {
        match self {
            Self::Tape(instructions) => instructions.is_empty(),
            Self::Single(instruction) => instruction.is_none(),
            Self::Sequence { index, length, .. } => index >= length,
        }
    }
}

/// A stack of instruction frames awaiting printing or measurement.
pub(crate) trait Queue<'a> {
    /// The underlying frame stack.
    type Stack: Stack<QueueFrame<'a>>;

    /// Return the underlying frame stack.
    fn stack(&self) -> &Self::Stack;

    /// Return the underlying frame stack mutably.
    fn stack_mut(&mut self) -> &mut Self::Stack;

    /// Pop the next instruction.
    fn pop(&mut self) -> Option<Instruction<'a>> {
        loop {
            // advance the current frame without moving it through the frame vector
            let frame = self.stack_mut().top_mut()?;
            let instruction = frame.next();
            let is_empty = frame.is_empty();

            // discard exhausted frames before returning their final instruction
            if is_empty {
                self.stack_mut().pop();
            }

            if instruction.is_some() {
                return instruction;
            }
        }
    }

    /// Return the next instruction without entering nested slices.
    fn peek_shallow(&self) -> Option<Instruction<'a>> {
        let mut frame = self.stack().top()?.clone();

        frame.next()
    }

    /// Return the next instruction after entering leading nested slices.
    fn peek(&self) -> Option<Instruction<'a>> {
        let mut instruction = self.peek_shallow();

        while instruction.is_some_and(|instruction| instruction.opcode() == Opcode::Slice) {
            let slice = instruction?.slice();
            instruction = slice.iter().next();
        }

        instruction
    }

    /// Queue encoded instructions before the existing work.
    fn push_slice(&mut self, instructions: InstructionSlice<'a>) {
        if !instructions.is_empty() {
            self.stack_mut().push(QueueFrame::tape(instructions));
        }
    }

    /// Queue one instruction before the existing work.
    fn push_instruction(&mut self, instruction: Instruction<'a>) {
        self.stack_mut().push(QueueFrame::Single(Some(instruction)));
    }

    /// Queue two synthetic instructions.
    fn push_pair(&mut self, instructions: [Instruction<'a>; 2]) {
        let mut storage = [Instruction::line(crate::format::LineMode::Soft); 3];
        storage[..2].copy_from_slice(&instructions);
        self.stack_mut().push(QueueFrame::Sequence {
            instructions: storage,
            index: 0,
            length: 2,
        });
    }

    /// Queue three synthetic instructions.
    fn push_triple(&mut self, instructions: [Instruction<'a>; 3]) {
        self.stack_mut().push(QueueFrame::Sequence {
            instructions,
            index: 0,
            length: 3,
        });
    }

    /// Remove the most recently queued frame.
    fn pop_frame(&mut self) -> Option<QueueFrame<'a>> {
        self.stack_mut().pop()
    }

    /// Skip content through the matching structural end instruction.
    fn skip_content(&mut self, kind: FormatTagKind) -> PrintResult<()>
    where
        Self: Sized,
    {
        for instruction in self.iter_content(kind) {
            instruction?;
        }

        Ok(())
    }

    /// Iterate through content before the matching structural end instruction.
    fn iter_content<'q>(&'q mut self, kind: FormatTagKind) -> QueueContentIterator<'a, 'q, Self>
    where
        Self: Sized,
    {
        QueueContentIterator::new(self, kind)
    }
}

/// The instructions awaiting final printing.
#[derive(Debug, Default, Clone)]
pub(crate) struct PrintQueue<'a> {
    /// The instruction frames in traversal order.
    frames: Vec<QueueFrame<'a>>,
}

impl<'a> PrintQueue<'a> {
    /// Create one queue from root instructions.
    pub(crate) fn new(instructions: InstructionSlice<'a>) -> Self {
        let mut frames = Vec::new();

        if !instructions.is_empty() {
            frames.push(QueueFrame::tape(instructions));
        }

        Self { frames }
    }
}

impl<'a> Queue<'a> for PrintQueue<'a> {
    type Stack = Vec<QueueFrame<'a>>;

    fn stack(&self) -> &Self::Stack {
        &self.frames
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.frames
    }
}

/// A restorable view over one print queue used during fit measurement.
#[must_use]
#[derive(Debug)]
pub(crate) struct FitsQueue<'a, 'print> {
    /// The borrowed print frames plus frames pushed during measurement.
    stack: StackedStack<'print, QueueFrame<'a>>,
}

impl<'a, 'print> FitsQueue<'a, 'print> {
    /// Create one fit queue and reuse previous temporary storage.
    pub(super) fn new(print_queue: &'print PrintQueue<'a>, saved: Vec<QueueFrame<'a>>) -> Self {
        Self {
            stack: StackedStack::with_vec(&print_queue.frames, saved),
        }
    }

    /// Take reusable temporary storage.
    pub(super) fn take_storage(&mut self) -> Vec<QueueFrame<'a>> {
        self.stack.take_vec()
    }
}

impl<'a, 'print> Queue<'a> for FitsQueue<'a, 'print> {
    type Stack = StackedStack<'print, QueueFrame<'a>>;

    fn stack(&self) -> &Self::Stack {
        &self.stack
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.stack
    }
}

/// An iterator over one structural scope in a queue.
pub(crate) struct QueueContentIterator<'a, 'q, Q: Queue<'a>> {
    /// The queue being consumed.
    queue: &'q mut Q,
    /// The structural scope kind.
    kind: FormatTagKind,
    /// The nested scope depth.
    depth: usize,
    /// The instruction lifetime.
    lifetime: PhantomData<&'a ()>,
}

impl<'a, 'q, Q> QueueContentIterator<'a, 'q, Q>
where
    Q: Queue<'a>,
{
    /// Create one structural content iterator.
    fn new(queue: &'q mut Q, kind: FormatTagKind) -> Self {
        Self {
            queue,
            kind,
            depth: 1,
            lifetime: PhantomData,
        }
    }
}

impl<'a, Q> Iterator for QueueContentIterator<'a, '_, Q>
where
    Q: Queue<'a>,
{
    type Item = PrintResult<Instruction<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.depth == 0 {
            return None;
        }

        // enter leading nested instruction slices
        let mut instruction = self.queue.pop();
        while instruction.is_some_and(|instruction| instruction.opcode() == Opcode::Slice) {
            let slice = instruction?.slice();
            self.queue.push_slice(slice);
            instruction = self.queue.pop();
        }

        // update matching structural depth
        let Some(instruction) = instruction else {
            self.depth = 0;

            return Some(missing_end_tag(self.kind));
        };
        if let Some(tag) = instruction.tag()
            && tag.kind() == self.kind
        {
            if tag.is_start() {
                self.depth += 1;
            } else {
                self.depth -= 1;

                if self.depth == 0 {
                    return None;
                }
            }
        }

        Some(Ok(instruction))
    }
}

impl<'a, Q> FusedIterator for QueueContentIterator<'a, '_, Q> where Q: Queue<'a> {}

/// A predicate that terminates fit measurement at a selected instruction.
pub(super) trait FitsEndPredicate {
    /// Return whether measurement should stop before this instruction.
    fn is_end(&mut self, instruction: Instruction<'_>) -> PrintResult<bool>;
}

/// A predicate that measures through the end of the queue.
pub(super) struct AllPredicate;

impl FitsEndPredicate for AllPredicate {
    fn is_end(&mut self, _instruction: Instruction<'_>) -> PrintResult<bool> {
        Ok(false)
    }
}

/// A predicate that measures exactly one fill entry.
#[derive(Debug, Default)]
pub(super) enum SingleEntryPredicate {
    /// Measure one potentially nested entry.
    #[default]
    Entry,
    /// Measure one nested entry scope.
    Nested { depth: usize },
    /// Stop measurement.
    Done,
}

impl SingleEntryPredicate {
    /// Return whether this predicate completed its entry.
    pub(super) const fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl FitsEndPredicate for SingleEntryPredicate {
    fn is_end(&mut self, instruction: Instruction<'_>) -> PrintResult<bool> {
        match self {
            Self::Done => Ok(true),
            Self::Entry => match instruction.tag() {
                Some(InstructionTag::StartEntry) => {
                    *self = Self::Nested { depth: 1 };
                    Ok(false)
                }
                None if instruction.opcode() == Opcode::Slice => Ok(false),
                _ => invalid_start_tag(FormatTagKind::Entry, Some(instruction)),
            },
            Self::Nested { depth } => match instruction.tag() {
                Some(InstructionTag::StartEntry) => {
                    *depth += 1;
                    Ok(false)
                }
                Some(InstructionTag::EndEntry) => {
                    *depth -= 1;
                    let is_done = *depth == 0;

                    if is_done {
                        *self = Self::Done;
                    }

                    Ok(is_done)
                }
                _ => Ok(false),
            },
        }
    }
}
