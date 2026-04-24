use crate::Value;
use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

use crate::interpreter::StackAllocation;

/// Durable call frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameImage {
    /// The logical frame layout for this activation.
    pub frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The block index in the function.
    pub block_index: usize,
    /// The program counter within the current block.
    pub resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub transfer: Option<engine::ControlTransfer>,
    /// The number of SSA values in this frame.
    pub value_count: usize,
    /// The number of locals in this frame.
    pub local_count: usize,
    /// The captured frame slots.
    pub slots: Vec<Value>,
    /// The captured stack allocations.
    pub(crate) stack_allocations: Vec<Option<StackAllocation>>,
    /// The captured callable environment.
    pub environment: Value,
}
