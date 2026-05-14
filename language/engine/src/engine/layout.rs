use serde::{Deserialize, Serialize};

use crate::{FrameLayout, FrameState, Materialization, Safepoint, StackMap};

/// Runtime value layout id inside one program layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValueLayoutId(pub u32);

/// Runtime layout tables for one compiled program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramLayout {
    /// Frame states by id.
    pub frame_states: Vec<FrameState>,
    /// Frame layouts by id.
    pub frame_layouts: Vec<FrameLayout>,
    /// Safepoints by id.
    pub safepoints: Vec<Safepoint>,
    /// Stack maps by id.
    pub stack_maps: Vec<StackMap>,
    /// Materializations by id.
    pub materializations: Vec<Materialization>,
}
