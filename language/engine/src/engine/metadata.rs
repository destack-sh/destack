use serde::{Deserialize, Serialize};

use crate::{FrameEntry, FrameLayout, FrameState, Materialization, Safepoint, StackMap};

/// One program type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub u32);

/// Program execution metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Frame layouts by id.
    pub frame_layouts: Vec<FrameLayout>,
    /// Frame states by id.
    pub frame_states: Vec<FrameState>,
    /// Frame entries by id.
    pub frame_entries: Vec<FrameEntry>,
    /// Safepoints by id.
    pub safepoints: Vec<Safepoint>,
    /// Stack maps by id.
    pub stack_maps: Vec<StackMap>,
    /// Materializations by id.
    pub materializations: Vec<Materialization>,
}
