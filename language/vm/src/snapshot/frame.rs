use destack_heap::{Value, ValueBuffer};
use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

/// Durable call frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterFrameImage {
    /// The logical frame layout for this activation.
    pub frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The entry block of the function.
    pub entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// The block index in the function.
    pub block_index: usize,
    /// The program counter within the current block.
    pub resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub transfer: Option<engine::FrameTransfer>,
    /// The base offset into the value stack.
    pub value_base: usize,
    /// The number of SSA values in this frame.
    pub value_count: usize,
    /// The base offset into the local stack.
    pub local_base: usize,
    /// The number of locals in this frame.
    pub local_count: usize,
    /// The captured stack allocated value buffers.
    pub stack_values: Vec<Option<ValueBuffer>>,
    /// The captured function environment.
    pub environment: Value,
}
