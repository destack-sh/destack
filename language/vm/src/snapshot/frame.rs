use destack_heap::{HeapCell, Value};
use destack_mir as mir;
use serde::{Deserialize, Serialize};

/// Durable call frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSnapshot {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The entry block of the function.
    pub entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// The threaded block index in the function.
    pub block_index: usize,
    /// The program counter within the current block.
    pub resume_pc: usize,
    /// The base offset into the value stack.
    pub value_base: usize,
    /// The number of SSA values in this frame.
    pub value_count: usize,
    /// The base offset into the local stack.
    pub local_base: usize,
    /// The number of locals in this frame.
    pub local_count: usize,
    /// The captured stack allocated cells.
    pub stack_cells: Vec<HeapCell>,
    /// The captured closure environment.
    pub closure_env: Value,
    /// The captured return destination.
    pub return_destination: mir::Value,
}
