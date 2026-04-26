use std::collections::HashMap;
use std::ptr::NonNull;

use {destack_engine as engine, destack_mir as mir};

use super::{ArgumentRange, Instruction, MovePair, MoveRange};

/// Switch case.
#[derive(Clone, Debug)]
pub(crate) struct SwitchCase {
    /// Match value.
    pub value: i64,
    /// Target block.
    pub target: u32,
    /// Block parameter moves.
    pub moves: MoveRange,
}

/// Lowered basic block.
#[derive(Clone, Debug)]
pub(crate) struct Block {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Instructions including terminator.
    pub instructions: Vec<Instruction>,
    /// MIR instruction boundary for each lowered PC in this block.
    pub mir_instruction_offsets: Vec<u32>,
}

/// Lowered function with predecoded dispatch metadata.
#[derive(Clone, Debug)]
pub(crate) struct Function {
    /// The logical frame layout for this function.
    pub frame_layout: engine::FrameLayoutId,
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// All blocks.
    pub blocks: Vec<Block>,
    /// Pool of argument values referenced by ranges.
    pub argument_pool: Vec<mir::Value>,
    /// Pool of value move pairs referenced by ranges.
    pub move_pool: Vec<MovePair>,
}

/// Lowered function registry owned by one program.
#[derive(Debug)]
pub(crate) struct FunctionTable {
    /// Lowered functions by dense index.
    functions: Vec<Function>,
    /// Callable target by function id.
    target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
}

/// Program call target resolved for one function id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallTarget {
    /// The function id names one imported callable.
    Import,
    /// The function id names one lowered callable.
    Local(u32),
}

impl FunctionTable {
    /// Build a lowered function table from lowered functions and callable targets.
    pub(super) fn new(
        functions: Vec<Function>,
        target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
    ) -> Self {
        Self {
            functions,
            target_by_id,
        }
    }

    /// Resolve the callable target for the given function id.
    pub(crate) fn resolve(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<CallTarget> {
        self.target_by_id.get(&func_id).copied()
    }

    /// Resolve a lowered function index for the given id.
    pub(crate) fn index_for(&self, func_id: mir::LocalNodeId<mir::Function>) -> Option<u32> {
        match self.resolve(func_id)? {
            CallTarget::Local(index) => Some(index),
            CallTarget::Import => None,
        }
    }

    /// Get a lowered function pointer by index.
    pub(crate) fn get_ptr_by_index(&self, index: u32) -> Option<NonNull<Function>> {
        self.functions.get(index as usize).map(NonNull::from)
    }

    /// Resolve one lowered function pointer by function id.
    pub(crate) fn get_ptr_for(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<NonNull<Function>> {
        let index = self.index_for(func_id)?;
        self.get_ptr_by_index(index)
    }
}
