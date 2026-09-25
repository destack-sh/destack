use std::num::NonZeroU32;

use tspp_unicode::UnicodeWidthChar;

use super::{
    ArenaVec, FormatElement, Instruction, InstructionIter, InstructionSlice, InstructionTag, Opcode,
};

/// The measurement policy used to select one best-fitting variant.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum BestFittingMode {
    /// Measure content through its first effective line break.
    #[default]
    FirstLine,

    /// Measure every line in the candidate layout.
    AllLines,
}

/// Alternative layouts ordered from most flat to most expanded.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BestFittingVariants<'a>(&'a [InstructionSlice<'a>]);

impl<'a> BestFittingVariants<'a> {
    /// Create best-fitting variants from one arena vector.
    pub(crate) fn from_vec_unchecked(variants: ArenaVec<'a, InstructionSlice<'a>>) -> Self {
        debug_assert!(
            variants.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );
        Self(variants.into_slice())
    }

    /// Create best-fitting variants from stable storage.
    pub(crate) fn from_slice_unchecked(variants: &'a [InstructionSlice<'a>]) -> Self {
        debug_assert!(variants.len() >= 2);

        Self(variants)
    }

    /// Return the most expanded variant.
    pub(crate) fn most_expanded(self) -> InstructionSlice<'a> {
        debug_assert!(self.0.len() >= 2);

        // safety: every constructor requires at least two variants
        unsafe { *self.0.get_unchecked(self.0.len() - 1) }
    }

    /// Return the variants from most flat to most expanded.
    pub(crate) const fn as_slice(self) -> &'a [InstructionSlice<'a>] {
        self.0
    }

    /// Iterate over variants from most flat to most expanded.
    pub(crate) fn iter(self) -> impl DoubleEndedIterator<Item = InstructionSlice<'a>> + 'a {
        self.0.iter().copied()
    }

    /// Return the most flat variant.
    pub(crate) fn most_flat(self) -> InstructionSlice<'a> {
        debug_assert!(self.0.len() >= 2);

        // safety: every constructor requires at least two variants
        unsafe { *self.0.get_unchecked(0) }
    }
}

/// Layout queries over FIR instructions.
pub trait FormatLayout {
    /// Return whether this content is guaranteed to break.
    fn will_break(&self) -> bool;

    /// Return whether this content directly contains a breakable line.
    fn may_directly_break(&self) -> bool;

    /// Return the single-line width when every instruction in this slice is measurable.
    fn single_line_width(&self) -> Option<u32>;
}

impl super::instruction::LineMode {
    /// Return whether this line always breaks.
    pub const fn will_break(self) -> bool {
        matches!(self, Self::Hard | Self::Empty)
    }
}

impl FormatLayout for FormatElement<'_> {
    fn will_break(&self) -> bool {
        match self {
            FormatElement::Line(mode) => mode.will_break(),
            FormatElement::ExpandParent => true,
            FormatElement::Text { width, .. } | FormatElement::FileSlice { width, .. } => {
                width.is_multiline()
            }
            FormatElement::Slice(instructions) => instructions.will_break(),
            FormatElement::BestFitting { variants, .. } => variants.most_flat().will_break(),
            FormatElement::Tag(crate::format::FormatTag::StartGroup(group)) => {
                !group.mode().is_flat()
            }
            _ => false,
        }
    }

    fn may_directly_break(&self) -> bool {
        match self {
            FormatElement::Line(_) => true,
            FormatElement::Text { width, .. } | FormatElement::FileSlice { width, .. } => {
                width.is_multiline()
            }
            FormatElement::Slice(instructions) => instructions.may_directly_break(),
            FormatElement::BestFitting { variants, .. } => {
                variants.most_flat().may_directly_break()
            }
            _ => false,
        }
    }

    fn single_line_width(&self) -> Option<u32> {
        match self {
            FormatElement::Space | FormatElement::Line(super::LineMode::SoftOrSpace) => Some(1),
            FormatElement::Token { text } => Some(text.len() as u32),
            FormatElement::Text { width, .. } | FormatElement::FileSlice { width, .. } => {
                Some(width.width()?.value())
            }
            FormatElement::Slice(instructions) => instructions.single_line_width(),
            FormatElement::BestFitting { variants, .. } => variants.most_flat().single_line_width(),
            _ => None,
        }
    }
}

impl FormatLayout for super::InstructionTape<'_> {
    fn will_break(&self) -> bool {
        instructions_will_break(InstructionTraversal::from_tape(self))
    }

    fn may_directly_break(&self) -> bool {
        instructions_may_directly_break(InstructionTraversal::from_tape(self))
    }

    fn single_line_width(&self) -> Option<u32> {
        instructions_single_line_width(InstructionTraversal::from_tape(self))
    }
}

impl FormatLayout for InstructionSlice<'_> {
    fn will_break(&self) -> bool {
        instructions_will_break(InstructionTraversal::new(*self))
    }

    fn may_directly_break(&self) -> bool {
        instructions_may_directly_break(InstructionTraversal::new(*self))
    }

    fn single_line_width(&self) -> Option<u32> {
        instructions_single_line_width(InstructionTraversal::new(*self))
    }
}

/// Return whether traversed instructions are guaranteed to break.
fn instructions_will_break(mut instructions: InstructionTraversal<'_, '_>) -> bool {
    let mut ignore_line_suffix_depth = 0usize;

    while let Some(instruction) = instructions.next() {
        let opcode = instruction.opcode();

        match opcode {
            Opcode::StartLineSuffix => ignore_line_suffix_depth += 1,
            Opcode::EndLineSuffix => {
                ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
            }
            Opcode::Slice if ignore_line_suffix_depth == 0 => {
                instructions.enter(instruction.slice());
            }
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines
                if ignore_line_suffix_depth == 0 =>
            {
                let (variants, _) = instruction.best_fitting();
                instructions.enter(variants.most_flat());
            }
            Opcode::HardLine | Opcode::EmptyLine => {
                return true;
            }
            Opcode::ExpandParent if ignore_line_suffix_depth == 0 => {
                return true;
            }
            Opcode::Text
                if ignore_line_suffix_depth == 0 && instruction.text().1.is_multiline() =>
            {
                return true;
            }
            Opcode::FileSlice
                if ignore_line_suffix_depth == 0 && instruction.file_slice().1.is_multiline() =>
            {
                return true;
            }
            Opcode::StartGroup if ignore_line_suffix_depth == 0 => {
                // safety: the opcode encodes one group start tag
                let tag = unsafe { instruction.tag().unwrap_unchecked() };
                if matches!(tag, InstructionTag::StartGroup(_, mode) if !mode.is_flat()) {
                    return true;
                }
            }
            _ => {}
        }
    }

    debug_assert_eq!(ignore_line_suffix_depth, 0, "unclosed line suffix");

    false
}

/// Return whether traversed instructions directly contain a breakable line.
fn instructions_may_directly_break(mut instructions: InstructionTraversal<'_, '_>) -> bool {
    let mut ignore_line_suffix_depth = 0usize;

    while let Some(instruction) = instructions.next() {
        let opcode = instruction.opcode();

        match opcode {
            Opcode::StartLineSuffix => ignore_line_suffix_depth += 1,
            Opcode::EndLineSuffix => {
                ignore_line_suffix_depth = ignore_line_suffix_depth.saturating_sub(1);
            }
            Opcode::Slice if ignore_line_suffix_depth == 0 => {
                instructions.enter(instruction.slice());
            }
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines
                if ignore_line_suffix_depth == 0 =>
            {
                let (variants, _) = instruction.best_fitting();
                instructions.enter(variants.most_flat());
            }
            Opcode::SoftLine | Opcode::SoftOrSpaceLine | Opcode::HardLine | Opcode::EmptyLine
                if ignore_line_suffix_depth == 0 =>
            {
                return true;
            }
            Opcode::Text
                if ignore_line_suffix_depth == 0 && instruction.text().1.is_multiline() =>
            {
                return true;
            }
            Opcode::FileSlice
                if ignore_line_suffix_depth == 0 && instruction.file_slice().1.is_multiline() =>
            {
                return true;
            }
            _ => {}
        }
    }

    debug_assert_eq!(ignore_line_suffix_depth, 0, "unclosed line suffix");

    false
}

/// Return the single-line width of traversed measurable instructions.
fn instructions_single_line_width(mut instructions: InstructionTraversal<'_, '_>) -> Option<u32> {
    let mut width = 0u32;

    while let Some(instruction) = instructions.next() {
        let instruction_width = match instruction.opcode() {
            Opcode::Space | Opcode::SoftOrSpaceLine => 1,
            Opcode::Token => instruction.token_text().len() as u32,
            Opcode::Text => instruction.text().1.width()?.value(),
            Opcode::FileSlice => instruction.file_slice().1.width()?.value(),
            Opcode::Slice => {
                instructions.enter(instruction.slice());
                continue;
            }
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                let (variants, _) = instruction.best_fitting();
                instructions.enter(variants.most_flat());
                continue;
            }
            _ => return None,
        };
        width = width.saturating_add(instruction_width);
    }

    Some(width)
}

/// One explicit depth-first traversal over instruction slices.
struct InstructionTraversal<'tape, 'a> {
    current: InstructionCursor<'tape, 'a>,
    parents: Vec<InstructionCursor<'tape, 'a>>,
}

impl<'a> InstructionTraversal<'a, 'a> {
    /// Create one traversal over an instruction slice.
    fn new(instructions: InstructionSlice<'a>) -> Self {
        Self {
            current: InstructionCursor::Slice(instructions.iter()),
            parents: Vec::new(),
        }
    }
}

impl<'tape, 'a> InstructionTraversal<'tape, 'a> {
    /// Create one traversal over a growing instruction tape.
    fn from_tape(instructions: &'tape super::InstructionTape<'a>) -> Self {
        Self {
            current: InstructionCursor::Tape(instructions.instructions().iter()),
            parents: Vec::new(),
        }
    }

    /// Enter one nested instruction slice before continuing the current slice.
    fn enter(&mut self, instructions: InstructionSlice<'a>) {
        let next = InstructionCursor::Slice(instructions.iter());
        let parent = std::mem::replace(&mut self.current, next);
        self.parents.push(parent);
    }

    /// Return the next instruction without implicitly entering nested slices.
    fn next(&mut self) -> Option<Instruction<'a>> {
        loop {
            // read or leave the current instruction slice
            let Some(instruction) = self.current.next() else {
                self.current = self.parents.pop()?;
                continue;
            };

            return Some(instruction);
        }
    }
}

/// One active instruction source in a layout query.
enum InstructionCursor<'tape, 'a> {
    /// The borrowed growing tape at the traversal root.
    Tape(std::slice::Iter<'tape, Instruction<'a>>),
    /// One stable nested instruction slice.
    Slice(InstructionIter<'a>),
}

impl<'a> Iterator for InstructionCursor<'_, 'a> {
    type Item = Instruction<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Tape(instructions) => instructions.next().copied(),
            Self::Slice(instructions) => instructions.next(),
        }
    }
}

/// One compact single-line display width.
///
/// The stored value adds one so `TextWidth` and `Option<Width>` remain four bytes.
/// Widths above `u32::MAX - 1` saturate because they exceed every supported print width.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Width(NonZeroU32);

impl Width {
    /// Create one display width.
    pub(crate) const fn new(width: u32) -> Self {
        Self(NonZeroU32::MIN.saturating_add(width))
    }

    /// Return the represented display width.
    pub const fn value(self) -> u32 {
        self.0.get() - 1
    }
}

/// The precomputed display width or multiline state of one text value.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TextWidth {
    /// One single-line display width.
    Width(Width),
    /// Text containing at least one line feed.
    Multiline,
}

impl TextWidth {
    /// Measure text using the configured tab width.
    pub fn from_text(text: &str, indent_width: u8) -> Self {
        let mut width = 0u32;

        for character in text.chars() {
            let character_width = match character {
                '\t' => indent_width,
                '\n' => return Self::Multiline,
                character => character.terminal_display_width(),
            };
            width = width.saturating_add(u32::from(character_width));
        }

        Self::Width(Width::new(width))
    }

    /// Return the single-line width when this text is not multiline.
    pub const fn width(self) -> Option<Width> {
        match self {
            TextWidth::Width(width) => Some(width),
            TextWidth::Multiline => None,
        }
    }

    /// Return whether this text contains a line feed.
    pub(crate) const fn is_multiline(self) -> bool {
        matches!(self, TextWidth::Multiline)
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{
        Allocator, FormatElement, FormatState, FormatTag, Formatter, LineMode, SimpleFormatContext,
    };

    use super::FormatLayout;

    /// Query deeply nested FIR slices without consuming call stack.
    #[test]
    fn test_queries_nested_instruction_slices() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_tspp(), &allocator);

        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Line(LineMode::Hard));
        let mut breaking = formatter.into_tape().into_slice();

        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Token { text: "x" });
        let mut flat = formatter.into_tape().into_slice();

        for _ in 0..100_000 {
            let mut formatter = Formatter::new(&mut state);
            formatter.write_element(FormatElement::Slice(breaking));
            breaking = formatter.into_tape().into_slice();

            let mut formatter = Formatter::new(&mut state);
            formatter.write_element(FormatElement::Slice(flat));
            flat = formatter.into_tape().into_slice();
        }

        assert!(breaking.will_break());
        assert!(breaking.may_directly_break());
        assert_eq!(breaking.single_line_width(), None);
        assert!(!flat.will_break());
        assert!(!flat.may_directly_break());
        assert_eq!(flat.single_line_width(), Some(1));
    }

    /// Ignore nested line breaks owned by line suffixes.
    #[test]
    fn test_queries_skip_nested_line_suffix_content() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_tspp(), &allocator);
        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Line(LineMode::Hard));
        let suffix = formatter.into_tape().into_slice();

        let mut formatter = Formatter::new(&mut state);
        formatter.write_element(FormatElement::Tag(FormatTag::StartLineSuffix));
        formatter.write_element(FormatElement::Slice(suffix));
        formatter.write_element(FormatElement::Tag(FormatTag::EndLineSuffix));
        let instructions = formatter.into_tape();

        assert!(!instructions.will_break());
        assert!(!instructions.may_directly_break());
        assert_eq!(instructions.single_line_width(), None);
    }
}
