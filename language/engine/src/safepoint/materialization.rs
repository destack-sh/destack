use crate::{FrameLayoutId, ResumePointId, SafepointId, ValueLocation};

/// The identifier for one materialization-map table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MaterializationMapId(pub u32);

/// One logical slot materialization recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MaterializationValue {
    /// One value copied from one logical frame slot.
    FrameSlot(u32),
    /// One value loaded from one physical location.
    Location(ValueLocation),
    /// One undefined slot that is not materialized at this boundary.
    Undefined,
}

/// One fixed-slot logical frame reconstruction recipe.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaterializationFrame {
    /// The logical frame layout to rebuild.
    pub frame_layout: FrameLayoutId,
    /// The resume point to restore for the rebuilt frame.
    pub resume_point: ResumePointId,
    /// The materialization recipe for each logical slot in layout order.
    pub slots: Vec<MaterializationValue>,
}

/// One fixed-slot logical frame materialization recipe for one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MaterializationMap {
    /// The materialization-map identifier.
    pub id: MaterializationMapId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// The frame reconstruction recipes ordered from outermost to innermost.
    pub frames: Vec<MaterializationFrame>,
}
