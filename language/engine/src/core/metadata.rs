use crate::{FrameLayout, MaterializationMap, Safepoint, StackMap};

/// Backend-neutral metadata for one executable program.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProgramMetadata {
    /// Frame layouts by dense id.
    pub frame_layouts: Vec<FrameLayout>,
    /// Safepoints by dense id.
    pub safepoints: Vec<Safepoint>,
    /// Stack maps by dense id.
    pub stack_maps: Vec<StackMap>,
    /// Materialization maps by dense id.
    pub materialization_maps: Vec<MaterializationMap>,
}
