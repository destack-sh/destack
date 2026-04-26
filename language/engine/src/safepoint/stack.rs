use crate::{FrameLayoutId, FrameRegionId, SafepointId, Value};

/// The identifier for one stack-map table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StackMapId(pub u32);

/// One physical register identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RegisterId(pub u16);

/// One stack byte location in native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackLocation {
    /// The byte offset from the chosen frame base.
    pub offset: i32,
}

/// One physical value location at one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ValueLocation {
    /// One register location.
    Register(RegisterId),
    /// One stack location.
    Stack(StackLocation),
    /// One constant value materialized directly from metadata.
    Constant(Value),
    /// One dead region with no live value at this safepoint.
    Dead,
}

/// One region location entry inside one frame stack map.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackMapRegion {
    /// The logical region inside the frame layout.
    pub region: FrameRegionId,
    /// The physical location for the region value.
    pub location: ValueLocation,
}

/// One frame-level physical location map.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackMapFrame {
    /// The logical frame layout reconstructed by this frame entry.
    pub frame_layout: FrameLayoutId,
    /// The region locations for this frame.
    pub regions: Vec<StackMapRegion>,
}

/// One native physical root-location map for one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StackMap {
    /// The stack-map identifier.
    pub id: StackMapId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// The physical frame maps ordered from outermost to innermost.
    pub frames: Vec<StackMapFrame>,
}
