use serde::{Deserialize, Serialize};

use destack_mir as mir;

/// Materialized frame captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedFrame {
    /// The captured frame state.
    pub frame_state: mir::FrameStateId,
    /// The caller return frame state.
    pub return_state: Option<mir::FrameStateId>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}
