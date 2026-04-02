use destack_heap::Value;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use {destack_engine as engine, destack_mir as mir};

use crate::interpreter::StackAllocation;

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
    /// The captured stack allocations.
    pub(crate) stack_allocations: Vec<Option<StackAllocation>>,
    /// The captured function environment.
    pub environment: Value,
}

impl InterpreterFrameImage {
    /// Return the owned bytes for this durable interpreter frame.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.stack_allocations.capacity() * size_of::<Option<StackAllocation>>();

        owned_bytes
    }
}
