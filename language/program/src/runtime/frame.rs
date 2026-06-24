use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_mir as mir;

/// Durable frame image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameImage {
    /// The captured frame state.
    pub frame_state: mir::FrameStateId,
    /// The caller return frame state.
    pub return_state: Option<mir::FrameStateId>,
    /// The byte offset inside the captured stack image.
    pub stack_offset: usize,
    /// The captured frame byte width.
    pub byte_len: usize,
}
