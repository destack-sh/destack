use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FrameRegionId, SafepointId, Value};

/// One stack map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StackMapId(pub u32);

/// One machine register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegisterId(pub u16);

/// One stack byte location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackLocation {
    /// The byte offset from the chosen frame base.
    pub offset: i32,
}

/// One value location at a safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueLocation {
    /// One register location.
    Register(RegisterId),
    /// One stack location.
    Stack(StackLocation),
    /// One constant value.
    Constant(Value),
    /// One dead region.
    Dead,
}

/// One frame region location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMapRegion {
    /// The frame region.
    pub region: FrameRegionId,
    /// The region location.
    pub location: ValueLocation,
}

/// One frame stack map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMapFrame {
    /// The frame layout.
    pub frame_layout: FrameLayoutId,
    /// The frame region locations.
    pub regions: Vec<StackMapRegion>,
}

/// Value locations at one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMap {
    /// The stack map id.
    pub id: StackMapId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// Frame maps from outermost to innermost.
    pub frames: Vec<StackMapFrame>,
}
