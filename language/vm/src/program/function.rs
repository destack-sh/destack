use std::collections::HashMap;
use std::ptr::NonNull;

use {destack_engine as engine, destack_mir as mir};

use super::{ArgumentRange, Instruction, MovePair, MoveRange};

/// Lowered function with executable code and frame metadata.
#[derive(Clone, Debug)]
pub(crate) struct Function {
    /// Original MIR function id.
    pub mir_function: mir::LocalNodeId<mir::Function>,
    /// The logical frame layout for this function.
    pub frame_layout: engine::FrameLayoutId,
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// Contiguous instruction code.
    pub code: Vec<Instruction>,
    /// Lowered block ranges.
    pub blocks: Vec<Block>,
    /// Pool of argument values referenced by ranges.
    pub argument_pool: Vec<mir::Value>,
    /// Pool of value move pairs referenced by ranges.
    pub move_pool: Vec<MovePair>,
}

impl Function {
    /// Return one block's instruction count.
    #[inline(always)]
    pub(crate) fn block_len(&self, block: u32) -> Option<usize> {
        self.blocks
            .get(block as usize)
            .map(|block| block.len as usize)
    }
}

/// Lowered function registry owned by one program.
#[derive(Debug)]
pub(crate) struct FunctionTable {
    /// Lowered functions by dense index.
    functions: Vec<Function>,
    /// Callable target by function id.
    target_by_id: HashMap<mir::LocalNodeId<mir::Function>, CallTarget>,
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

    /// Return a lowered function pointer by dense index.
    pub(crate) fn pointer(&self, index: u32) -> Option<NonNull<Function>> {
        self.functions.get(index as usize).map(NonNull::from)
    }

    /// Resolve one lowered function pointer by function id.
    pub(crate) fn pointer_for(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<NonNull<Function>> {
        let index = self.index_for(func_id)?;
        self.pointer(index)
    }

    /// Return one lowered function by function id.
    pub(crate) fn function_for(
        &self,
        func_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<&Function> {
        let index = self.index_for(func_id)?;

        self.functions.get(index as usize)
    }
}

/// Program call target for one function id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallTarget {
    /// The function id names one imported callable.
    Import,
    /// The function id names one lowered callable.
    Local(u32),
}

/// Lowered basic block position inside one function.
#[derive(Clone, Debug)]
pub(crate) struct Block {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// First instruction in the function code.
    pub start: u32,
    /// Number of instructions in this block.
    pub len: u32,
    /// Source MIR boundary for each lowered PC in this block.
    pub source_boundary_by_pc: Vec<u32>,
}

/// Instruction bytes emitted for one block during lowering.
#[derive(Clone, Debug)]
pub(crate) struct BlockCode {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Instructions including terminator.
    pub instructions: Vec<Instruction>,
    /// Source MIR boundary for each lowered PC in this block.
    pub source_boundary_by_pc: Vec<u32>,
}

/// One lowered switch case.
#[derive(Clone, Debug)]
pub(crate) struct SwitchCase {
    /// Match value.
    pub value: i128,
    /// Target block.
    pub target: u32,
    /// Block parameter moves.
    pub moves: MoveRange,
}
