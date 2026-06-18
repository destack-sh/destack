use serde::{Deserialize, Serialize};

use crate::{FrameStateId, MaterializationId, StackMapId};

/// One safepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SafepointId(pub u32);

/// Point where roots and reconstruction are known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Safepoint {
    /// The safepoint id.
    pub id: SafepointId,
    /// The frame state at this safepoint.
    pub frame_state: FrameStateId,
    /// The stack map at this safepoint.
    pub stack_map: StackMapId,
    /// The materialization recipe when present.
    pub materialization: Option<MaterializationId>,
}
