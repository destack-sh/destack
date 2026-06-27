use destack_serde::Reflect;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FunctionId};

use super::{ArgumentRange, Instruction, MovePair, MoveRange, MoveSlot};

/// Lowered function with executable code and frame tables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Function {
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

impl Function {
    /// Return one block's instruction count.
    #[inline(always)]
    pub fn block_len(&self, block: u32) -> Option<usize> {
        self.blocks
            .get(block as usize)
            .map(|block| block.len as usize)
    }
}

/// Lowered function registry owned by one program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FunctionTable {
    /// Lowered functions by dense index.
    functions: Vec<Function>,
    /// Call target by function id.
    target_by_id: HashMap<FunctionId, CallTarget>,
}

impl FunctionTable {
    /// Build a lowered function table from lowered functions and call targets.
    pub fn new(functions: Vec<Function>, target_by_id: HashMap<FunctionId, CallTarget>) -> Self {
        Self {
            functions,
            target_by_id,
        }
    }

    /// Return the call target for the given function id.
    pub fn call_target(&self, func_id: FunctionId) -> Option<CallTarget> {
        self.target_by_id.get(&func_id).copied()
    }

    /// Return a lowered local function index for the given function id.
    pub fn local_index(&self, func_id: FunctionId) -> Option<u32> {
        match self.call_target(func_id)? {
            CallTarget::Local(index) => Some(index),
            CallTarget::Import => None,
        }
    }

    /// Return a lowered function by dense index.
    pub fn function_by_index(&self, index: u32) -> Option<&Function> {
        self.functions.get(index as usize)
    }

    /// Return one lowered function by function id.
    pub fn function_by_id(&self, func_id: FunctionId) -> Option<&Function> {
        let index = self.local_index(func_id)?;

        self.functions.get(index as usize)
    }
}

/// Program call target for one function id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CallTarget {
    /// The function id names one imported function.
    Import,
    /// The function id names one lowered function.
    Local(u32),
}

/// Lowered basic block position inside one function.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Block {
    /// First instruction in the function code.
    pub start: u32,
    /// Number of instructions in this block.
    pub len: u32,
}

/// One lowered switch case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SwitchCase {
    /// Match value.
    pub value: i128,
    /// Target block.
    pub target: u32,
    /// Block parameter moves.
    pub moves: MoveRange,
}
