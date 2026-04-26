use crate::{FrameLayoutId, FunctionId, MaterializationMapId, ResumePointId, StackMapId};

/// The identifier for one safepoint table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SafepointId(pub u32);

/// One logical safepoint in the shared execution model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Safepoint {
    /// The safepoint identifier.
    pub id: SafepointId,
    /// The owning function.
    pub function: FunctionId,
    /// The owning frame layout.
    pub frame_layout: FrameLayoutId,
    /// The resume point reached at this safepoint.
    pub resume_point: ResumePointId,
    /// The physical root-location table when present.
    pub stack_map: Option<StackMapId>,
    /// The logical frame materialization recipe when present.
    pub materialization_map: Option<MaterializationMapId>,
}
