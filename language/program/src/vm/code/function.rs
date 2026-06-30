use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FunctionId};

use super::{ArgumentRange, Instruction, MovePair, MoveRange, MoveSlot};

/// Lowered VM function registry owned by one program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionTable {
    /// Lowered functions by dense local index.
    functions: SectionSlice<Function>,
    /// Flattened instruction entries.
    code: SectionSlice<Instruction>,
    /// Flattened block entries.
    blocks: SectionSlice<Block>,
    /// Flattened argument move slots.
    argument_pool: SectionSlice<MoveSlot>,
    /// Flattened move pairs.
    move_pool: SectionSlice<MovePair>,
    /// Dense call target entries keyed by program function id.
    call_targets: SectionSlice<Optional<CallTarget>>,
}

impl FunctionTable {
    /// Build a section-backed lowered function table.
    pub fn pack(
        sections: &mut SectionPacker,
        functions: Vec<FunctionBuilder>,
        call_targets: Vec<Option<CallTarget>>,
    ) -> Self {
        let mut function_entries = Vec::with_capacity(functions.len());
        let mut code = EntryStore::new();
        let mut blocks = EntryStore::new();
        let mut argument_pool = EntryStore::new();
        let mut move_pool = EntryStore::new();

        // flatten variable function payloads
        for function in functions {
            let entry = Function {
                function: function.function,
                frame_layout: function.frame_layout,
                parameters: function.parameters,
                entry: function.entry,
                code: code.append(function.code),
                blocks: blocks.append(function.blocks),
                argument_pool: argument_pool.append(function.argument_pool),
                move_pool: move_pool.append(function.move_pool),
            };

            function_entries.push(entry);
        }

        let call_targets = call_targets
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();

        Self {
            functions: sections.insert(function_entries),
            code: sections.insert(code.into_entries()),
            blocks: sections.insert(blocks.into_entries()),
            argument_pool: sections.insert(argument_pool.into_entries()),
            move_pool: sections.insert(move_pool.into_entries()),
            call_targets: sections.insert(call_targets),
        }
    }

    /// Return the call target for the given function id.
    pub fn call_target(
        &self,
        sections: SectionImage<'_>,
        function: FunctionId,
    ) -> Option<CallTarget> {
        sections
            .entries(self.call_targets)
            .get(function.index())
            .and_then(|target| target.get())
    }

    /// Return a lowered local function index for the given function id.
    pub fn local_index(&self, sections: SectionImage<'_>, function: FunctionId) -> Option<u32> {
        self.call_target(sections, function)?.local_index()
    }

    /// Return a lowered function by dense index.
    pub fn function_by_index<'a>(
        &self,
        sections: SectionImage<'a>,
        index: u32,
    ) -> Option<FunctionCode<'a>> {
        let function = *sections.entries(self.functions).get(index as usize)?;

        Some(self.code_for(sections, function))
    }

    /// Return one lowered function by function id.
    pub fn function_by_id<'a>(
        &self,
        sections: SectionImage<'a>,
        function: FunctionId,
    ) -> Option<FunctionCode<'a>> {
        let index = self.local_index(sections, function)?;

        self.function_by_index(sections, index)
    }

    /// Return all lowered function entries.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Function] {
        sections.entries(self.functions)
    }

    /// Borrow one lowered function code view.
    fn code_for<'a>(&self, sections: SectionImage<'a>, function: Function) -> FunctionCode<'a> {
        FunctionCode {
            function,
            code: sections.range(self.code, function.code),
            blocks: sections.range(self.blocks, function.blocks),
            argument_pool: sections.range(self.argument_pool, function.argument_pool),
            move_pool: sections.range(self.move_pool, function.move_pool),
        }
    }
}

/// Build-time lowered function payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionBuilder {
    /// Runtime function id.
    pub function: FunctionId,
    /// The logical frame layout for this function.
    pub frame_layout: FrameLayoutId,
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// Contiguous instruction code.
    pub code: Vec<Instruction>,
    /// Lowered block ranges.
    pub blocks: Vec<Block>,
    /// Pool of argument frame slots referenced by ranges.
    pub argument_pool: Vec<MoveSlot>,
    /// Pool of value move pairs referenced by ranges.
    pub move_pool: Vec<MovePair>,
}

/// Lowered function entry stored in a program section.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Function {
    /// Runtime function id.
    pub function: FunctionId,
    /// The logical frame layout for this function.
    pub frame_layout: FrameLayoutId,
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// Instruction entries for this function.
    pub code: EntryRange<Instruction>,
    /// Block entries for this function.
    pub blocks: EntryRange<Block>,
    /// Argument slot entries for this function.
    pub argument_pool: EntryRange<MoveSlot>,
    /// Move pair entries for this function.
    pub move_pool: EntryRange<MovePair>,
}

/// Borrowed VM function code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunctionCode<'a> {
    /// Lowered function entry.
    pub function: Function,
    /// Contiguous instruction code.
    pub code: &'a [Instruction],
    /// Lowered block ranges.
    pub blocks: &'a [Block],
    /// Pool of argument frame slots referenced by ranges.
    pub argument_pool: &'a [MoveSlot],
    /// Pool of value move pairs referenced by ranges.
    pub move_pool: &'a [MovePair],
}

impl FunctionCode<'_> {
    /// Return one block's instruction count.
    #[inline(always)]
    pub fn block_len(&self, block: u32) -> Option<usize> {
        self.blocks
            .get(block as usize)
            .map(|block| block.len as usize)
    }
}

/// Program call target for one function id.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallTarget {
    /// Target kind.
    kind: CallTargetKind,
    /// Local function slot when this is a local target.
    index: u32,
}

impl CallTarget {
    /// The function id names one imported function.
    pub const IMPORT: Self = Self {
        kind: CallTargetKind::Import,
        index: 0,
    };

    /// Create one local function target.
    pub const fn local(index: u32) -> Self {
        Self {
            kind: CallTargetKind::Local,
            index,
        }
    }

    /// Return whether this target names an import.
    pub const fn is_import(self) -> bool {
        matches!(self.kind, CallTargetKind::Import)
    }

    /// Return the local function slot.
    pub const fn local_index(self) -> Option<u32> {
        if matches!(self.kind, CallTargetKind::Local) {
            Some(self.index)
        } else {
            None
        }
    }
}

/// Program call target kind.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
enum CallTargetKind {
    /// Function is imported.
    #[default]
    Import = 0,
    /// Function is defined in local VM code.
    Local = 1,
}

/// Lowered basic block position inside one function.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Block {
    /// First instruction in the function code.
    pub start: u32,
    /// Number of instructions in this block.
    pub len: u32,
}

/// One lowered switch case.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SwitchCase {
    /// Match value.
    pub value: i128,
    /// Target block.
    pub target: u32,
    /// Block parameter moves.
    pub moves: MoveRange,
}

// SAFETY: VM function entries are repr(C), Copy, and contain only section entries.
unsafe impl SectionEntry for Function {}
unsafe impl SectionEntry for CallTarget {}
unsafe impl SectionEntry for CallTargetKind {}
unsafe impl SectionEntry for Block {}
unsafe impl SectionEntry for SwitchCase {}
