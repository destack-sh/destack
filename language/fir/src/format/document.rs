use rustc_hash::FxHashSet;

use crate::format::{
    ExpansionRow, FitsExpandedIndex, FitsExpandedState, GroupIndex, GroupState, InstructionSlice,
    Opcode,
};

/// A completed language-independent formatting document.
#[derive(Debug)]
pub struct Document<'a> {
    /// The root formatting instructions.
    instructions: InstructionSlice<'a>,
    /// The layout state for logical groups.
    groups: Vec<GroupState>,
    /// The layout state for fits-expanded scopes.
    fits_expanded: Vec<FitsExpandedState>,
}

impl<'a> Document<'a> {
    /// Create one completed document and apply its precomputed expansion effects.
    pub(crate) fn new(
        instructions: InstructionSlice<'a>,
        mut groups: Vec<GroupState>,
        mut fits_expanded: Vec<FitsExpandedState>,
    ) -> Self {
        // apply expansion effects accumulated while building the root tape
        let expansion = instructions.expansion();
        for row in expansion.rows() {
            match row {
                ExpansionRow::Group(index) => groups[index.as_usize()].expand(),
                ExpansionRow::FitsExpanded(index) => {
                    fits_expanded[index.as_usize()].is_expanded = true;
                }
            }
        }

        Self {
            instructions,
            groups,
            fits_expanded,
        }
    }

    /// Return the root formatting instructions.
    pub(crate) fn instructions(&self) -> InstructionSlice<'a> {
        self.instructions
    }

    /// Measure the encoded shape of this document.
    pub fn measure(&self) -> DocumentStats {
        let top_level_instructions = self.instructions.len() as u64;
        let mut stats = DocumentStats {
            top_level_instructions,
            stored_instructions: top_level_instructions,
            stored_slices: 1,
            metadata_bytes: self.instructions.metadata_bytes() as u64,
            ..DocumentStats::default()
        };
        let mut stored_slices: FxHashSet<*const ()> = FxHashSet::default();
        stored_slices.insert(self.instructions.as_ptr().cast());
        let mut pending = vec![(self.instructions, InstructionOwner::Root, true)];

        while let Some((instructions, owner, is_stored)) = pending.pop() {
            for instruction in instructions.iter() {
                stats.recursive_instructions += 1;

                match owner {
                    InstructionOwner::Root => {}
                    InstructionOwner::Slice => stats.slice_instructions += 1,
                    InstructionOwner::BestFitting => stats.best_fitting_instructions += 1,
                }

                match instruction.opcode() {
                    Opcode::Space => stats.spaces += 1,
                    Opcode::SoftLine
                    | Opcode::SoftOrSpaceLine
                    | Opcode::HardLine
                    | Opcode::EmptyLine => stats.lines += 1,
                    Opcode::ExpandParent => stats.expand_parents += 1,
                    Opcode::Token => stats.tokens += 1,
                    Opcode::Text => stats.texts += 1,
                    Opcode::SourcePosition => stats.source_positions += 1,
                    Opcode::FileSlice => stats.file_slices += 1,
                    Opcode::LineSuffixBoundary => stats.line_suffix_boundaries += 1,
                    Opcode::Slice => {
                        stats.slices += 1;
                        let slice = instruction.slice();
                        let is_stored = stored_slices.insert(slice.as_ptr().cast());
                        if is_stored {
                            stats.stored_instructions += slice.len() as u64;
                            stats.stored_slices += 1;
                            stats.metadata_bytes += slice.metadata_bytes() as u64;
                        }
                        pending.push((slice, InstructionOwner::Slice, is_stored));
                    }
                    Opcode::BestFittingFirstLine | Opcode::BestFittingAllLines => {
                        stats.best_fitting += 1;
                        let (variants, _) = instruction.best_fitting();
                        stats.best_fitting_variants += variants.as_slice().len() as u64;
                        if is_stored {
                            stats.metadata_bytes +=
                                std::mem::size_of_val(variants.as_slice()) as u64;
                        }

                        for variant in variants.iter() {
                            let is_stored = stored_slices.insert(variant.as_ptr().cast());
                            if is_stored {
                                stats.stored_instructions += variant.len() as u64;
                                stats.stored_slices += 1;
                                stats.metadata_bytes += variant.metadata_bytes() as u64;
                            }
                            pending.push((variant, InstructionOwner::BestFitting, is_stored));
                        }
                    }
                    _ => stats.tags += 1,
                }
            }
        }

        stats
    }

    /// Return one group row.
    pub(crate) fn group(&self, index: GroupIndex) -> &GroupState {
        &self.groups[index.as_usize()]
    }

    /// Return one fits-expanded row.
    pub(crate) fn fits_expanded(&self, index: FitsExpandedIndex) -> &FitsExpandedState {
        &self.fits_expanded[index.as_usize()]
    }
}

/// Structural counts for one encoded formatting document.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct DocumentStats {
    /// The root instruction count.
    pub top_level_instructions: u64,
    /// The recursive instruction count including nested slices and variants.
    pub recursive_instructions: u64,
    /// The uniquely stored instruction count.
    pub stored_instructions: u64,
    /// The uniquely stored instruction slice count.
    pub stored_slices: u64,
    /// The instruction descriptor, variant table, and expansion row byte count.
    pub metadata_bytes: u64,
    /// The space instruction count.
    pub spaces: u64,
    /// The line instruction count.
    pub lines: u64,
    /// The expand-parent instruction count.
    pub expand_parents: u64,
    /// The static token instruction count.
    pub tokens: u64,
    /// The borrowed text instruction count.
    pub texts: u64,
    /// The source position instruction count.
    pub source_positions: u64,
    /// The file slice instruction count.
    pub file_slices: u64,
    /// The line suffix boundary instruction count.
    pub line_suffix_boundaries: u64,
    /// The nested slice instruction count.
    pub slices: u64,
    /// The instructions reached through nested slices.
    pub slice_instructions: u64,
    /// The best-fitting instruction count.
    pub best_fitting: u64,
    /// The best-fitting variant count.
    pub best_fitting_variants: u64,
    /// The instructions reached through best-fitting variants.
    pub best_fitting_instructions: u64,
    /// The structural tag instruction count.
    pub tags: u64,
}

impl DocumentStats {
    /// Return the encoded instruction byte count.
    pub const fn instruction_bytes(self) -> u64 {
        self.stored_instructions * std::mem::size_of::<crate::format::Instruction<'_>>() as u64
    }

    /// Return the stored FIR payload byte count.
    pub const fn bytes(self) -> u64 {
        self.instruction_bytes() + self.metadata_bytes
    }
}

/// The parent storage that led to one measured instruction tape.
#[derive(Debug, Clone, Copy)]
enum InstructionOwner {
    /// The root document tape.
    Root,
    /// One nested reusable slice.
    Slice,
    /// One best-fitting variant.
    BestFitting,
}
