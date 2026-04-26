use crate::{FrameLayoutId, FrameRegionId, ResumePointId, SafepointId, ValueLocation};

/// The identifier for one materialization-map table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MaterializationMapId(pub u32);

/// One logical frame region materialization recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MaterializationValue {
    /// One value copied from one logical frame region.
    FrameRegion(FrameRegionId),
    /// One value loaded from one physical location.
    Location(ValueLocation),
    /// One undefined region that is not materialized at this boundary.
    Undefined,
}

/// One logical frame reconstruction recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaterializationFrame {
    /// The logical frame layout to rebuild.
    pub frame_layout: FrameLayoutId,
    /// The resume point to restore for the rebuilt frame.
    pub resume_point: ResumePointId,
    /// The materialization recipe for each logical region in layout order.
    pub regions: Vec<MaterializationValue>,
}

/// One frame materialization recipe for one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaterializationMap {
    /// The materialization-map identifier.
    pub id: MaterializationMapId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// The frame reconstruction recipes ordered from outermost to innermost.
    pub frames: Vec<MaterializationFrame>,
}
