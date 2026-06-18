use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FrameSlotId, SafepointId, Value};

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

/// Source for one live frame slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotSource {
    /// The destination frame slot.
    pub slot: FrameSlotId,
    /// Where the slot value comes from.
    pub source: ValueSource,
}

/// Origin of one frame slot value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueSource {
    /// Value from one logical frame slot.
    Slot(FrameSlotId),
    /// Value from one machine register.
    Register(RegisterId),
    /// Value from one machine stack location.
    Stack(StackLocation),
    /// One constant value.
    Constant(Value),
}

/// One frame stack map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMapFrame {
    /// The frame layout.
    pub frame_layout: FrameLayoutId,
    /// Sources for live slots.
    pub sources: Vec<SlotSource>,
}

/// Slot sources at one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackMap {
    /// The stack map id.
    pub id: StackMapId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// Frame maps from outermost to innermost.
    pub frames: Vec<StackMapFrame>,
}
