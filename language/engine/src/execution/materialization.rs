use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FrameRegionId, FrameStateId, SafepointId, ValueLocation};

/// One materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterializationId(pub u32);

/// Source for one reconstructed frame region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterializationValue {
    /// Value from one frame region.
    FrameRegion(FrameRegionId),
    /// Value from one stack map location.
    Location(ValueLocation),
    /// One undefined region.
    Undefined,
}

/// Recipe for reconstructing one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationFrame {
    /// The reconstructed frame layout.
    pub frame_layout: FrameLayoutId,
    /// The reconstructed frame state.
    pub frame_state: FrameStateId,
    /// Region sources in layout order.
    pub regions: Vec<MaterializationValue>,
}

/// Recipe for reconstructing one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Materialization {
    /// The materialization id.
    pub id: MaterializationId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// The reconstructed frames from outermost to innermost.
    pub frames: Vec<MaterializationFrame>,
}
