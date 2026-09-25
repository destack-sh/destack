use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use tspp_source::ByteRange;

use crate::format::{
    Allocator, ArenaVec, BestFittingMode, BestFittingVariants, FitsExpandedIndex, FormatTag,
    FormatTagKind, GroupIndex, GroupMode, TextWidth,
};

/// One compact instruction tied to its borrowed formatter storage.
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct Instruction<'a> {
    /// The opcode and compact payload.
    header: u64,
    /// The pointer or large scalar operand.
    operand: u64,
    /// The lifetime of any pointer payload.
    lifetime: PhantomData<&'a ()>,
}

const _: () = assert!(std::mem::size_of::<Instruction<'_>>() == 16);

/// The bit width reserved for an instruction opcode.
const OPCODE_BITS: u32 = 8;
/// The mask selecting an instruction opcode.
const OPCODE_MASK: u64 = (1 << OPCODE_BITS) - 1;
/// The bit width reserved for one compact text width.
const TEXT_WIDTH_BITS: u32 = 9;
/// A single-line width larger than every configurable print width.
const OVERFLOW_TEXT_WIDTH: u64 = 1 << 8;
/// The compact marker for multiline text.
const MULTILINE_TEXT_WIDTH: u64 = OVERFLOW_TEXT_WIDTH + 1;

/// The printing behavior of one line instruction.
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LineMode {
    /// Break when the enclosing group expands.
    Soft,
    /// Break when the enclosing group expands, or print one space otherwise.
    SoftOrSpace,
    /// Always break the line.
    Hard,
    /// Always print one empty line.
    Empty,
}

/// One language-independent formatting operation.
#[derive(Clone, Copy, PartialEq)]
pub enum FormatElement<'a> {
    /// One space.
    Space,
    /// One conditional or unconditional line break.
    Line(LineMode),
    /// Force the enclosing group to expand.
    ExpandParent,
    /// Static ASCII text without line breaks or tabs.
    Token { text: &'static str },
    /// Borrowed text with precomputed display width.
    Text { text: &'a str, width: TextWidth },
    /// The source position for subsequent output.
    SourcePosition { source: u32 },
    /// One verbatim file-local source range.
    FileSlice { range: ByteRange, width: TextWidth },
    /// A boundary that flushes pending line suffixes.
    LineSuffixBoundary,
    /// One reusable arena-backed instruction slice.
    Slice(InstructionSlice<'a>),
    /// Alternative layouts ordered from most flat to most expanded.
    BestFitting {
        /// The available layouts.
        variants: BestFittingVariants<'a>,
        /// The measurement policy used to choose a layout.
        mode: BestFittingMode,
    },
    /// One structural formatting tag.
    Tag(FormatTag),
}

impl Debug for FormatElement<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Space => formatter.write_str("Space"),
            Self::Line(mode) => formatter.debug_tuple("Line").field(mode).finish(),
            Self::ExpandParent => formatter.write_str("ExpandParent"),
            Self::Token { text } => formatter.debug_tuple("Token").field(text).finish(),
            Self::Text { text, .. } => formatter.debug_tuple("Text").field(text).finish(),
            Self::SourcePosition { source } => formatter
                .debug_tuple("SourcePosition")
                .field(source)
                .finish(),
            Self::FileSlice { range, width } => formatter
                .debug_tuple("FileSlice")
                .field(range)
                .field(width)
                .finish(),
            Self::LineSuffixBoundary => formatter.write_str("LineSuffixBoundary"),
            Self::Slice(slice) => slice.fmt(formatter),
            Self::BestFitting { variants, mode } => formatter
                .debug_struct("BestFitting")
                .field("variants", variants)
                .field("mode", mode)
                .finish(),
            Self::Tag(tag) => formatter.debug_tuple("Tag").field(tag).finish(),
        }
    }
}

/// The expansion effects produced by one instruction slice.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExpansionEffects<'a> {
    /// Whether this slice expands its enclosing group.
    expands_parent: bool,
    /// The document rows expanded inside this slice.
    rows: &'a [ExpansionRow],
}

impl<'a> ExpansionEffects<'a> {
    /// Return whether these effects expand an enclosing group.
    const fn expands_parent(self) -> bool {
        self.expands_parent
    }

    /// Return the document rows expanded by these effects.
    pub(crate) const fn rows(self) -> &'a [ExpansionRow] {
        self.rows
    }
}

/// One document row expanded while writing an instruction slice.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ExpansionRow {
    /// One logical group row.
    Group(GroupIndex),
    /// One fits-expanded row.
    FitsExpanded(FitsExpandedIndex),
}

/// The stable storage referenced by one instruction slice.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct InstructionStorage<'a> {
    /// The encoded instructions.
    instructions: &'a [Instruction<'a>],
    /// The expansion effects computed while writing the instructions, when present.
    expansion: Option<&'a ExpansionEffects<'a>>,
}

const _: () =
    assert!(std::mem::size_of::<InstructionStorage<'_>>() == 3 * std::mem::size_of::<usize>());

/// One expansion scope active while writing an instruction tape.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ExpansionScope {
    /// One regular or conditional group.
    Group {
        /// The dense document group row.
        index: GroupIndex,
        /// The initial group layout.
        mode: GroupMode,
    },
    /// One fits-expanded scope.
    FitsExpanded {
        /// The dense fits-expanded document row.
        index: FitsExpandedIndex,
    },
    /// One scope that blocks expansion propagation.
    Barrier,
}

/// One active expansion scope and its observed expansion state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct ExpansionFrame {
    /// The scope receiving expansion.
    scope: ExpansionScope,
    /// Whether content in this scope expands.
    is_expanded: bool,
}

/// Expansion effects accumulated while writing one instruction tape.
struct ExpansionTracker<'a> {
    /// Whether this tape expands its enclosing group.
    expands_parent: bool,
    /// The nested expansion scopes.
    frames: ArenaVec<'a, ExpansionFrame>,
    /// The document rows expanded by completed scopes or nested slices.
    rows: ArenaVec<'a, ExpansionRow>,
}

impl<'a> ExpansionTracker<'a> {
    /// Create one tracker with a root expansion scope.
    fn new(allocator: &'a Allocator) -> Self {
        Self {
            expands_parent: false,
            frames: ArenaVec::new_in(allocator),
            rows: ArenaVec::new_in(allocator),
        }
    }

    /// Observe one appended instruction.
    fn observe(&mut self, instruction: Instruction<'a>) {
        let opcode = instruction.opcode();

        match opcode {
            Opcode::Slice => {
                let effects = instruction.slice().expansion();
                self.merge_rows(effects);

                if effects.expands_parent() {
                    self.expand();
                }
            }
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                let (variants, _) = instruction.best_fitting();

                for variant in variants.iter() {
                    self.merge_rows(variant.expansion());
                }
            }
            Opcode::Text => {
                if instruction.text().1.is_multiline() {
                    self.expand();
                }
            }
            Opcode::FileSlice => {
                if instruction.file_slice().1.is_multiline() {
                    self.expand();
                }
            }
            Opcode::ExpandParent | Opcode::HardLine | Opcode::EmptyLine => self.expand(),
            Opcode::StartGroup
            | Opcode::EndGroup
            | Opcode::StartConditionalGroup
            | Opcode::EndConditionalGroup
            | Opcode::StartFitsExpanded
            | Opcode::EndFitsExpanded
            | Opcode::StartBestFitParenthesize
            | Opcode::EndBestFitParenthesize => {
                // safety: every matched opcode encodes one structural tag
                let tag = unsafe { instruction.tag().unwrap_unchecked() };
                self.observe_tag(tag);
            }
            _ => {}
        }
    }

    /// Observe one structural tag.
    fn observe_tag(&mut self, tag: InstructionTag) {
        match tag {
            InstructionTag::StartGroup(index, mode) => {
                self.push_scope(ExpansionScope::Group { index, mode });
            }
            InstructionTag::StartConditionalGroup(index) => {
                self.push_scope(ExpansionScope::Group {
                    index,
                    mode: GroupMode::Flat,
                });
            }
            InstructionTag::EndGroup | InstructionTag::EndConditionalGroup => {
                self.pop_group();
            }
            InstructionTag::StartFitsExpanded(index) => {
                self.push_scope(ExpansionScope::FitsExpanded { index });
            }
            InstructionTag::EndFitsExpanded => self.pop_fits_expanded(),
            InstructionTag::StartBestFitParenthesize(_) => {
                self.push_scope(ExpansionScope::Barrier);
            }
            InstructionTag::EndBestFitParenthesize => self.pop_barrier(),
            _ => {}
        }
    }

    /// Push one unexpanded scope.
    fn push_scope(&mut self, scope: ExpansionScope) {
        self.frames.push(ExpansionFrame {
            scope,
            is_expanded: false,
        });
    }

    /// Complete one group scope.
    fn pop_group(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let ExpansionScope::Group { index, mode } = frame.scope else {
            self.frames.push(frame);
            return;
        };

        // propagate the completed group into its enclosing scope
        if frame.is_expanded {
            self.rows.push(ExpansionRow::Group(index));
        }

        if frame.is_expanded || !mode.is_flat() {
            self.expand();
        }
    }

    /// Complete one fits-expanded scope without propagating it outward.
    fn pop_fits_expanded(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let ExpansionScope::FitsExpanded { index } = frame.scope else {
            self.frames.push(frame);
            return;
        };

        if frame.is_expanded {
            self.rows.push(ExpansionRow::FitsExpanded(index));
        }
    }

    /// Complete one expansion barrier.
    fn pop_barrier(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };

        if frame.scope != ExpansionScope::Barrier {
            self.frames.push(frame);
        }
    }

    /// Merge expanded document rows from one inserted instruction slice.
    fn merge_rows(&mut self, effects: ExpansionEffects<'a>) {
        for row in effects.rows {
            self.rows.push(*row);
        }
    }

    /// Mark the active scope as expanded.
    fn expand(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.is_expanded = true;
        } else {
            self.expands_parent = true;
        }
    }

    /// Store the completed effects in stable arena slices.
    fn finish(self) -> Option<ExpansionEffects<'a>> {
        let effects = ExpansionEffects {
            expands_parent: self.expands_parent,
            rows: self.rows.into_slice(),
        };

        if effects.expands_parent || !effects.rows.is_empty() {
            Some(effects)
        } else {
            None
        }
    }
}

/// One growing arena-backed instruction tape.
pub struct InstructionTape<'a> {
    /// The encoded instructions.
    instructions: ArenaVec<'a, Instruction<'a>>,
    /// The expansion effects under construction.
    expansion: ExpansionTracker<'a>,
}

impl<'a> InstructionTape<'a> {
    /// Create one empty instruction tape.
    pub(crate) fn new(allocator: &'a Allocator) -> Self {
        Self {
            instructions: ArenaVec::new_in(allocator),
            expansion: ExpansionTracker::new(allocator),
        }
    }

    /// Create one empty instruction tape with instruction capacity.
    pub(crate) fn with_capacity(capacity: usize, allocator: &'a Allocator) -> Self {
        Self {
            instructions: ArenaVec::with_capacity_in(capacity, allocator),
            expansion: ExpansionTracker::new(allocator),
        }
    }

    /// Append one encoded instruction.
    pub(crate) fn push(&mut self, instruction: Instruction<'a>) {
        self.expansion.observe(instruction);
        self.instructions.push(instruction);
    }

    /// Return the encoded instructions written so far.
    pub(crate) fn instructions(&self) -> &[Instruction<'a>] {
        &self.instructions
    }

    /// Return whether this tape contains no instructions.
    pub(crate) fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Store this tape as a stable instruction slice.
    pub(crate) fn into_slice(self) -> InstructionSlice<'a> {
        let allocator = self.instructions.allocator();
        let expansion = self
            .expansion
            .finish()
            .map(|effects| allocator.alloc(effects) as &ExpansionEffects<'a>);
        let storage = allocator.alloc(InstructionStorage {
            instructions: self.instructions.into_slice(),
            expansion,
        });

        InstructionSlice::new(storage)
    }

    /// Collapse this tape into zero, one, or nested formatting elements.
    pub fn collapse(self) -> Option<FormatElement<'a>> {
        if self.instructions.is_empty() {
            return None;
        }

        // preserve one directly representable element without another slice level
        if self.instructions.len() == 1 {
            let instruction = self.instructions[0];
            if let Some(element) = instruction.element() {
                return Some(element);
            }
        }

        Some(FormatElement::Slice(self.into_slice()))
    }
}

impl Debug for InstructionTape<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.instructions.iter())
            .finish()
    }
}

/// One decoded structural formatting instruction.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum InstructionTag {
    /// Start one indentation scope.
    StartIndent,
    /// End one indentation scope.
    EndIndent,
    /// Start one alignment scope.
    StartAlign(u8),
    /// End one alignment scope.
    EndAlign,
    /// Start one dedentation scope.
    StartDedent(crate::format::DedentMode),
    /// End one dedentation scope.
    EndDedent(crate::format::DedentMode),
    /// Start one logical group.
    StartGroup(GroupIndex, crate::format::GroupMode),
    /// End one logical group.
    EndGroup,
    /// Start one conditional logical group.
    StartConditionalGroup(GroupIndex),
    /// End one conditional logical group.
    EndConditionalGroup,
    /// Start conditional content.
    StartConditionalContent(crate::format::Condition),
    /// End conditional content.
    EndConditionalContent,
    /// Start conditional indentation.
    StartIndentIfGroupBreaks(crate::format::GroupId),
    /// End conditional indentation.
    EndIndentIfGroupBreaks(crate::format::GroupId),
    /// Start one fill scope.
    StartFill,
    /// End one fill scope.
    EndFill,
    /// Start one fill entry.
    StartEntry,
    /// End one fill entry.
    EndEntry,
    /// Start one line suffix.
    StartLineSuffix,
    /// End one line suffix.
    EndLineSuffix,
    /// Start one verbatim range.
    StartVerbatim(crate::format::VerbatimKind),
    /// End one verbatim range.
    EndVerbatim,
    /// Start one fits-expanded scope.
    StartFitsExpanded(FitsExpandedIndex),
    /// End one fits-expanded scope.
    EndFitsExpanded,
    /// Start one conditional parenthesized layout.
    StartBestFitParenthesize(Option<crate::format::GroupId>),
    /// End one conditional parenthesized layout.
    EndBestFitParenthesize,
}

impl InstructionTag {
    /// Return whether this instruction starts a structural scope.
    pub(crate) const fn is_start(self) -> bool {
        matches!(
            self,
            Self::StartIndent
                | Self::StartAlign(_)
                | Self::StartDedent(_)
                | Self::StartGroup(_, _)
                | Self::StartConditionalGroup(_)
                | Self::StartConditionalContent(_)
                | Self::StartIndentIfGroupBreaks(_)
                | Self::StartFill
                | Self::StartEntry
                | Self::StartLineSuffix
                | Self::StartVerbatim(_)
                | Self::StartFitsExpanded(_)
                | Self::StartBestFitParenthesize(_)
        )
    }

    /// Return the structural tag kind.
    pub(crate) const fn kind(self) -> FormatTagKind {
        match self {
            Self::StartIndent | Self::EndIndent => FormatTagKind::Indent,
            Self::StartAlign(_) | Self::EndAlign => FormatTagKind::Align,
            Self::StartDedent(_) | Self::EndDedent(_) => FormatTagKind::Dedent,
            Self::StartGroup(_, _) | Self::EndGroup => FormatTagKind::Group,
            Self::StartConditionalGroup(_) | Self::EndConditionalGroup => {
                FormatTagKind::ConditionalGroup
            }
            Self::StartConditionalContent(_) | Self::EndConditionalContent => {
                FormatTagKind::ConditionalContent
            }
            Self::StartIndentIfGroupBreaks(_) | Self::EndIndentIfGroupBreaks(_) => {
                FormatTagKind::IndentIfGroupBreaks
            }
            Self::StartFill | Self::EndFill => FormatTagKind::Fill,
            Self::StartEntry | Self::EndEntry => FormatTagKind::Entry,
            Self::StartLineSuffix | Self::EndLineSuffix => FormatTagKind::LineSuffix,
            Self::StartVerbatim(_) | Self::EndVerbatim => FormatTagKind::Verbatim,
            Self::StartFitsExpanded(_) | Self::EndFitsExpanded => FormatTagKind::FitsExpanded,
            Self::StartBestFitParenthesize(_) | Self::EndBestFitParenthesize => {
                FormatTagKind::BestFitParenthesize
            }
        }
    }
}

/// One reusable arena-backed instruction slice.
#[derive(Clone, Copy, Eq)]
pub struct InstructionSlice<'a> {
    /// The stable instruction storage.
    storage: &'a InstructionStorage<'a>,
    /// The first visible instruction.
    start: usize,
}

const _: () =
    assert!(std::mem::size_of::<InstructionSlice<'_>>() == 2 * std::mem::size_of::<usize>());

impl<'a> InstructionSlice<'a> {
    /// Create one complete instruction slice.
    const fn new(storage: &'a InstructionStorage<'a>) -> Self {
        Self { storage, start: 0 }
    }

    /// Return an iterator over this slice.
    pub(crate) fn iter(self) -> InstructionIter<'a> {
        InstructionIter::new(self)
    }

    /// Return whether this slice is empty.
    pub(crate) const fn is_empty(self) -> bool {
        self.start == self.storage.instructions.len()
    }

    /// Return the instruction count.
    pub(crate) const fn len(self) -> usize {
        self.storage.instructions.len() - self.start
    }

    /// Return the storage pointer identity.
    pub(crate) const fn as_ptr(self) -> *const () {
        self.storage as *const InstructionStorage<'a> as *const ()
    }

    /// Return the precomputed expansion effects.
    pub(crate) const fn expansion(self) -> ExpansionEffects<'a> {
        debug_assert!(self.start == 0);

        match self.storage.expansion {
            Some(expansion) => *expansion,
            None => ExpansionEffects {
                expands_parent: false,
                rows: &[],
            },
        }
    }

    /// Return the byte count of this slice's descriptor and expansion rows.
    pub(crate) const fn metadata_bytes(self) -> usize {
        debug_assert!(self.start == 0);

        let Some(expansion) = self.storage.expansion else {
            return std::mem::size_of::<InstructionStorage<'a>>();
        };

        std::mem::size_of::<InstructionStorage<'a>>()
            + std::mem::size_of::<ExpansionEffects<'a>>()
            + std::mem::size_of_val(expansion.rows)
    }
}

impl PartialEq for InstructionSlice<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && std::ptr::eq(self.storage, other.storage)
    }
}

impl Debug for InstructionSlice<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

/// A sequential iterator over one instruction slice.
#[derive(Debug, Clone)]
pub(crate) struct InstructionIter<'a> {
    /// The complete stable instruction storage.
    storage: &'a InstructionStorage<'a>,
    /// The remaining encoded instructions.
    instructions: &'a [Instruction<'a>],
}

impl<'a> InstructionIter<'a> {
    /// Create one iterator.
    pub(crate) fn new(instructions: InstructionSlice<'a>) -> Self {
        Self {
            storage: instructions.storage,
            instructions: &instructions.storage.instructions[instructions.start..],
        }
    }

    /// Return the remaining encoded slice.
    pub(crate) fn remaining(&self) -> InstructionSlice<'a> {
        let storage = self.storage;
        let start = storage.instructions.len() - self.instructions.len();

        InstructionSlice { storage, start }
    }

    /// Return whether this iterator has consumed its slice.
    pub(crate) const fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

impl<'a> Iterator for InstructionIter<'a> {
    type Item = Instruction<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (instruction, remaining) = self.instructions.split_first()?;
        self.instructions = remaining;

        Some(*instruction)
    }
}

/// The opcode stored in the low byte of each instruction header.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Opcode {
    Space,
    SoftLine,
    SoftOrSpaceLine,
    HardLine,
    EmptyLine,
    ExpandParent,
    Token,
    Text,
    SourcePosition,
    FileSlice,
    LineSuffixBoundary,
    Slice,
    BestFittingFirstLine,
    BestFittingAllLines,
    StartIndent,
    EndIndent,
    StartAlign,
    EndAlign,
    StartDedentLevel,
    EndDedentLevel,
    StartDedentRoot,
    EndDedentRoot,
    StartGroup,
    EndGroup,
    StartConditionalGroup,
    EndConditionalGroup,
    StartConditionalFlat,
    StartConditionalExpanded,
    EndConditionalContent,
    StartIndentIfGroupBreaks,
    EndIndentIfGroupBreaks,
    StartFill,
    EndFill,
    StartEntry,
    EndEntry,
    StartLineSuffix,
    EndLineSuffix,
    StartVerbatimBogus,
    StartVerbatimSuppressed,
    StartVerbatim,
    EndVerbatim,
    StartFitsExpanded,
    EndFitsExpanded,
    StartBestFitParenthesize,
    EndBestFitParenthesize,
}

impl Opcode {
    /// Encode this opcode and one compact payload.
    pub(crate) const fn encode<'a>(self, payload: u64) -> Instruction<'a> {
        self.encode_with(payload, 0)
    }

    /// Encode this opcode with one compact payload and one full operand.
    pub(crate) const fn encode_with<'a>(self, payload: u64, operand: u64) -> Instruction<'a> {
        Instruction {
            header: (payload << OPCODE_BITS) | self as u64,
            operand,
            lifetime: PhantomData,
        }
    }
}

impl<'a> Instruction<'a> {
    /// Return this instruction as one public formatting element when representable.
    fn element(self) -> Option<FormatElement<'a>> {
        match self.opcode() {
            Opcode::Space => Some(FormatElement::Space),
            Opcode::SoftLine | Opcode::SoftOrSpaceLine | Opcode::HardLine | Opcode::EmptyLine => {
                // safety: the matched opcodes are all line instructions
                let mode = unsafe { self.line_mode().unwrap_unchecked() };
                Some(FormatElement::Line(mode))
            }
            Opcode::ExpandParent => Some(FormatElement::ExpandParent),
            Opcode::Token => Some(FormatElement::Token {
                text: self.token_text(),
            }),
            Opcode::Text => {
                let (text, width) = self.text();
                Some(FormatElement::Text { text, width })
            }
            Opcode::SourcePosition => Some(FormatElement::SourcePosition {
                source: self.source_position(),
            }),
            Opcode::FileSlice => {
                let (range, width) = self.file_slice();
                Some(FormatElement::FileSlice { range, width })
            }
            Opcode::LineSuffixBoundary => Some(FormatElement::LineSuffixBoundary),
            Opcode::Slice => Some(FormatElement::Slice(self.slice())),
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                let (variants, mode) = self.best_fitting();
                Some(FormatElement::BestFitting { variants, mode })
            }
            _ => None,
        }
    }

    /// Return this instruction's opcode.
    #[inline(always)]
    pub(crate) fn opcode(self) -> Opcode {
        let opcode = (self.header & OPCODE_MASK) as u8;

        // safety: only Formatter emits instructions and every emitted opcode is valid
        unsafe { std::mem::transmute(opcode) }
    }

    /// Return this instruction's compact payload.
    #[inline(always)]
    const fn payload(self) -> u64 {
        self.header >> OPCODE_BITS
    }

    /// Return this instruction's full-width operand.
    #[inline(always)]
    const fn operand(self) -> u64 {
        self.operand
    }

    /// Return this instruction's line mode.
    #[inline]
    fn line_mode(self) -> Option<LineMode> {
        match self.opcode() {
            Opcode::SoftLine => Some(LineMode::Soft),
            Opcode::SoftOrSpaceLine => Some(LineMode::SoftOrSpace),
            Opcode::HardLine => Some(LineMode::Hard),
            Opcode::EmptyLine => Some(LineMode::Empty),
            _ => None,
        }
    }

    /// Return this instruction's static token.
    #[inline]
    pub(crate) fn token_text(self) -> &'static str {
        let pointer = self.operand() as usize as *const u8;
        let length = self.payload() as usize;

        // safety: Formatter only encodes static strings for token instructions
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };

        // safety: the source payload is a static Rust string
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Return this instruction's borrowed text and display width.
    #[inline]
    pub(crate) fn text(self) -> (&'a str, TextWidth) {
        let (length, width) = decode_text_layout(self.payload());

        (self.text_bytes(length), width)
    }

    /// Return this instruction's source position.
    pub(crate) const fn source_position(self) -> u32 {
        self.payload() as u32
    }

    /// Return this instruction's source range and display width.
    #[inline]
    pub(crate) fn file_slice(self) -> (ByteRange, TextWidth) {
        let operand = self.operand();
        let range = ByteRange {
            start: operand as u32,
            end: (operand >> 32) as u32,
        };

        (range, decode_width(self.payload()))
    }

    /// Return this instruction's nested instruction slice.
    #[inline]
    pub(crate) fn slice(self) -> InstructionSlice<'a> {
        let pointer = self.operand() as usize as *const InstructionStorage<'a>;

        // safety: Formatter encodes one stable arena instruction descriptor
        let storage = unsafe { &*pointer };

        InstructionSlice::new(storage)
    }

    /// Return this instruction's alternative layouts and measurement mode.
    #[inline]
    pub(crate) fn best_fitting(self) -> (BestFittingVariants<'a>, BestFittingMode) {
        let pointer = self.operand() as usize as *const InstructionSlice<'a>;
        let length = self.payload() as usize;

        // safety: Formatter encodes one stable arena slice and its exact variant count
        let variants = unsafe { std::slice::from_raw_parts(pointer, length) };
        let variants = BestFittingVariants::from_slice_unchecked(variants);
        let mode = if self.opcode() == Opcode::BestFittingFirstLine {
            BestFittingMode::FirstLine
        } else {
            BestFittingMode::AllLines
        };

        (variants, mode)
    }

    /// Return this instruction's structural tag.
    #[inline]
    pub(crate) fn tag(self) -> Option<InstructionTag> {
        use crate::format::{Condition, DedentMode, PrintMode, VerbatimKind};

        let payload = self.payload();
        let tag = match self.opcode() {
            Opcode::StartIndent => InstructionTag::StartIndent,
            Opcode::EndIndent => InstructionTag::EndIndent,
            Opcode::StartAlign => InstructionTag::StartAlign(payload as u8),
            Opcode::EndAlign => InstructionTag::EndAlign,
            Opcode::StartDedentLevel => InstructionTag::StartDedent(DedentMode::Level),
            Opcode::EndDedentLevel => InstructionTag::EndDedent(DedentMode::Level),
            Opcode::StartDedentRoot => InstructionTag::StartDedent(DedentMode::Root),
            Opcode::EndDedentRoot => InstructionTag::EndDedent(DedentMode::Root),
            Opcode::StartGroup => {
                let index = GroupIndex::new(payload as u32);
                let mode = match (payload >> 32) as u8 {
                    0 => crate::format::GroupMode::Flat,
                    1 => crate::format::GroupMode::Expand,
                    2 => crate::format::GroupMode::Propagated,
                    // safety: Formatter encodes GroupMode's explicit discriminant
                    _ => unsafe { std::hint::unreachable_unchecked() },
                };

                InstructionTag::StartGroup(index, mode)
            }
            Opcode::EndGroup => InstructionTag::EndGroup,
            Opcode::StartConditionalGroup => {
                InstructionTag::StartConditionalGroup(GroupIndex::new(payload as u32))
            }
            Opcode::EndConditionalGroup => InstructionTag::EndConditionalGroup,
            Opcode::StartConditionalFlat | Opcode::StartConditionalExpanded => {
                let group_id = decode_group_id(payload as u32);
                let mode = if self.opcode() == Opcode::StartConditionalFlat {
                    PrintMode::Flat
                } else {
                    PrintMode::Expanded
                };

                InstructionTag::StartConditionalContent(Condition { mode, group_id })
            }
            Opcode::EndConditionalContent => InstructionTag::EndConditionalContent,
            Opcode::StartIndentIfGroupBreaks => {
                InstructionTag::StartIndentIfGroupBreaks(decode_required_group_id(payload as u32))
            }
            Opcode::EndIndentIfGroupBreaks => {
                InstructionTag::EndIndentIfGroupBreaks(decode_required_group_id(payload as u32))
            }
            Opcode::StartFill => InstructionTag::StartFill,
            Opcode::EndFill => InstructionTag::EndFill,
            Opcode::StartEntry => InstructionTag::StartEntry,
            Opcode::EndEntry => InstructionTag::EndEntry,
            Opcode::StartLineSuffix => InstructionTag::StartLineSuffix,
            Opcode::EndLineSuffix => InstructionTag::EndLineSuffix,
            Opcode::StartVerbatimBogus => InstructionTag::StartVerbatim(VerbatimKind::Bogus),
            Opcode::StartVerbatimSuppressed => {
                InstructionTag::StartVerbatim(VerbatimKind::Suppressed)
            }
            Opcode::StartVerbatim => InstructionTag::StartVerbatim(VerbatimKind::Verbatim {
                length: payload as u32,
            }),
            Opcode::EndVerbatim => InstructionTag::EndVerbatim,
            Opcode::StartFitsExpanded => {
                InstructionTag::StartFitsExpanded(FitsExpandedIndex::new(payload as u32))
            }
            Opcode::EndFitsExpanded => InstructionTag::EndFitsExpanded,
            Opcode::StartBestFitParenthesize => {
                InstructionTag::StartBestFitParenthesize(decode_group_id(payload as u32))
            }
            Opcode::EndBestFitParenthesize => InstructionTag::EndBestFitParenthesize,
            _ => return None,
        };

        Some(tag)
    }

    /// Return this instruction's tag kind when present.
    pub(crate) fn tag_kind(self) -> Option<FormatTagKind> {
        match self.opcode() {
            Opcode::StartIndent | Opcode::EndIndent => Some(FormatTagKind::Indent),
            Opcode::StartAlign | Opcode::EndAlign => Some(FormatTagKind::Align),
            Opcode::StartDedentLevel
            | Opcode::EndDedentLevel
            | Opcode::StartDedentRoot
            | Opcode::EndDedentRoot => Some(FormatTagKind::Dedent),
            Opcode::StartGroup | Opcode::EndGroup => Some(FormatTagKind::Group),
            Opcode::StartConditionalGroup | Opcode::EndConditionalGroup => {
                Some(FormatTagKind::ConditionalGroup)
            }
            Opcode::StartConditionalFlat
            | Opcode::StartConditionalExpanded
            | Opcode::EndConditionalContent => Some(FormatTagKind::ConditionalContent),
            Opcode::StartIndentIfGroupBreaks | Opcode::EndIndentIfGroupBreaks => {
                Some(FormatTagKind::IndentIfGroupBreaks)
            }
            Opcode::StartFill | Opcode::EndFill => Some(FormatTagKind::Fill),
            Opcode::StartEntry | Opcode::EndEntry => Some(FormatTagKind::Entry),
            Opcode::StartLineSuffix | Opcode::EndLineSuffix => Some(FormatTagKind::LineSuffix),
            Opcode::StartVerbatimBogus
            | Opcode::StartVerbatimSuppressed
            | Opcode::StartVerbatim
            | Opcode::EndVerbatim => Some(FormatTagKind::Verbatim),
            Opcode::StartFitsExpanded | Opcode::EndFitsExpanded => {
                Some(FormatTagKind::FitsExpanded)
            }
            Opcode::StartBestFitParenthesize | Opcode::EndBestFitParenthesize => {
                Some(FormatTagKind::BestFitParenthesize)
            }
            _ => None,
        }
    }

    /// Create one synthetic line instruction.
    pub(crate) const fn line(mode: LineMode) -> Self {
        match mode {
            LineMode::Soft => Opcode::SoftLine.encode(0),
            LineMode::SoftOrSpace => Opcode::SoftOrSpaceLine.encode(0),
            LineMode::Hard => Opcode::HardLine.encode(0),
            LineMode::Empty => Opcode::EmptyLine.encode(0),
        }
    }

    /// Create one synthetic static token instruction.
    pub(crate) fn token(text: &'static str) -> Self {
        Opcode::Token.encode_with(text.len() as u64, text.as_ptr() as usize as u64)
    }

    /// Create one synthetic indentation start.
    pub(crate) const fn start_indent() -> Self {
        Opcode::StartIndent.encode(0)
    }

    /// Create one synthetic line suffix end.
    pub(crate) const fn end_line_suffix() -> Self {
        Opcode::EndLineSuffix.encode(0)
    }

    /// Return one borrowed UTF-8 payload.
    fn text_bytes(self, length: usize) -> &'a str {
        let pointer = self.operand() as usize as *const u8;

        // safety: Formatter encodes one live string pointer and its exact UTF-8 byte length
        let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };

        // safety: the source payload is a Rust string
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }
}

impl Debug for Instruction<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Instruction")
            .field("opcode", &self.opcode())
            .finish_non_exhaustive()
    }
}

/// One instruction decoded for printer control flow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DecodedInstruction<'a> {
    /// One space.
    Space,
    /// One conditional or unconditional line break.
    Line(LineMode),
    /// Force the enclosing group to expand.
    ExpandParent,
    /// Static ASCII text without line breaks or tabs.
    Token(&'static str),
    /// Borrowed text with precomputed display width.
    Text { text: &'a str, width: TextWidth },
    /// The source position for subsequent output.
    SourcePosition(u32),
    /// One verbatim file-local source range.
    FileSlice { range: ByteRange, width: TextWidth },
    /// A boundary that flushes pending line suffixes.
    LineSuffixBoundary,
    /// One reusable arena-backed instruction slice.
    Slice(InstructionSlice<'a>),
    /// Alternative layouts ordered from most flat to most expanded.
    BestFitting {
        /// The available layouts.
        variants: BestFittingVariants<'a>,
        /// The measurement policy used to choose a layout.
        mode: BestFittingMode,
    },
    /// One structural formatting instruction.
    Tag(InstructionTag),
}

impl<'a> Instruction<'a> {
    /// Decode this instruction for printer control flow.
    #[inline(always)]
    pub(crate) fn decode(self) -> DecodedInstruction<'a> {
        match self.opcode() {
            Opcode::Space => DecodedInstruction::Space,
            Opcode::SoftLine | Opcode::SoftOrSpaceLine | Opcode::HardLine | Opcode::EmptyLine => {
                // safety: the matched opcodes are all line instructions
                let mode = unsafe { self.line_mode().unwrap_unchecked() };
                DecodedInstruction::Line(mode)
            }
            Opcode::ExpandParent => DecodedInstruction::ExpandParent,
            Opcode::Token => DecodedInstruction::Token(self.token_text()),
            Opcode::Text => {
                let (text, width) = self.text();
                DecodedInstruction::Text { text, width }
            }
            Opcode::SourcePosition => DecodedInstruction::SourcePosition(self.source_position()),
            Opcode::FileSlice => {
                let (range, width) = self.file_slice();
                DecodedInstruction::FileSlice { range, width }
            }
            Opcode::LineSuffixBoundary => DecodedInstruction::LineSuffixBoundary,
            Opcode::Slice => DecodedInstruction::Slice(self.slice()),
            Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                let (variants, mode) = self.best_fitting();
                DecodedInstruction::BestFitting { variants, mode }
            }
            _ => {
                // safety: every remaining opcode encodes one structural tag
                let tag = unsafe { self.tag().unwrap_unchecked() };
                DecodedInstruction::Tag(tag)
            }
        }
    }
}

/// Encode one text width for a configurable line width of at most 255.
pub(crate) const fn encode_width(width: TextWidth) -> u64 {
    match width.width() {
        Some(width) => {
            let width = width.value() as u64;

            if width < OVERFLOW_TEXT_WIDTH {
                width
            } else {
                OVERFLOW_TEXT_WIDTH
            }
        }
        None => MULTILINE_TEXT_WIDTH,
    }
}

/// Decode one compact text width.
fn decode_width(width: u64) -> TextWidth {
    if width == MULTILINE_TEXT_WIDTH {
        TextWidth::Multiline
    } else {
        TextWidth::Width(crate::format::Width::new(width as u32))
    }
}

/// Encode one borrowed text length and width.
pub(crate) const fn encode_text_layout(text: &str, width: TextWidth) -> u64 {
    let length = text.len() as u64;
    let width = encode_width(width);

    (length << TEXT_WIDTH_BITS) | width
}

/// Decode one borrowed text length and width.
fn decode_text_layout(layout: u64) -> (usize, TextWidth) {
    let width_mask = (1 << TEXT_WIDTH_BITS) - 1;
    let width = decode_width(layout & width_mask);
    let length = (layout >> TEXT_WIDTH_BITS) as usize;

    (length, width)
}

/// Encode one optional group identifier.
pub(crate) fn encode_group_id(group_id: Option<crate::format::GroupId>) -> u32 {
    group_id.map_or(0, u32::from)
}

/// Decode one optional group identifier.
fn decode_group_id(group_id: u32) -> Option<crate::format::GroupId> {
    std::num::NonZeroU32::new(group_id).map(crate::format::GroupId::new)
}

/// Decode one required group identifier.
fn decode_required_group_id(group_id: u32) -> crate::format::GroupId {
    // safety: the formatter only emits this opcode for a present GroupId
    let value = unsafe { std::num::NonZeroU32::new_unchecked(group_id) };

    crate::format::GroupId::new(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserve zero, one, and nested capture shapes when collapsing tapes.
    #[test]
    fn test_collapses_instruction_tapes() {
        let allocator = Allocator::default();
        let empty = InstructionTape::new(&allocator);
        assert_eq!(empty.collapse(), None);

        let mut single = InstructionTape::new(&allocator);
        single.push(Instruction::token("value"));
        assert_eq!(
            single.collapse(),
            Some(FormatElement::Token { text: "value" })
        );

        let mut child = InstructionTape::new(&allocator);
        child.push(Instruction::token("child"));
        let child = child.into_slice();
        let mut enclosing = InstructionTape::new(&allocator);
        enclosing.push(Opcode::Slice.encode_with(0, child.as_ptr() as usize as u64));
        assert_eq!(enclosing.collapse(), Some(FormatElement::Slice(child)));
    }

    /// Propagate expansion through deeply nested instruction slices without recursion.
    #[test]
    fn test_propagates_expansion_through_nested_slices() {
        let allocator = Allocator::default();
        let mut instructions = InstructionTape::new(&allocator);
        instructions.push(Opcode::HardLine.encode(0));
        let mut instructions = instructions.into_slice();

        for _ in 0..100_000 {
            let mut enclosing = InstructionTape::new(&allocator);
            enclosing.push(Opcode::Slice.encode_with(0, instructions.as_ptr() as usize as u64));
            instructions = enclosing.into_slice();
        }

        assert!(instructions.expansion().expands_parent());
    }
}
