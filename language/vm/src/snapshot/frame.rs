use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_mir as mir};

/// Durable call frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameImage {
    /// The logical frame layout.
    pub frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// The program counter within the current block.
    pub resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub transfer: Option<engine::ControlTransfer>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}
