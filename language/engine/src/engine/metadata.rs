use serde::{Deserialize, Serialize};

use crate::{FrameLayout, Materialization, Safepoint, StackMap};

/// One program layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayoutId(pub u32);

/// Program execution metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Frame layouts by id.
    pub frame_layouts: Vec<FrameLayout>,
    /// Safepoints by id.
    pub safepoints: Vec<Safepoint>,
    /// Stack maps by id.
    pub stack_maps: Vec<StackMap>,
    /// Materializations by id.
    pub materializations: Vec<Materialization>,
}
