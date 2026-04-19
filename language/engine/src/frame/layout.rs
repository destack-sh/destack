use destack_mir as mir;

use crate::{ControlTransfer, MaterializedValue, ResumePointId};

/// The identifier for one frame layout table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameLayoutId(pub u32);

/// The originating MIR entity represented by one frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameSlotSource {
    /// One SSA value slot.
    Value(mir::Value),
    /// One disaggregated hidden slot owned by one semantic value.
    DisaggregatedValue(mir::Value),
    /// One mutable local slot.
    Local(mir::LocalNodeId<mir::Local>),
    /// The function environment slot.
    Environment,
}

/// One logical slot in one frame layout.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameSlot {
    /// The originating MIR entity represented by this slot.
    pub source: FrameSlotSource,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// The logical layout of one function activation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameLayout {
    /// The layout identifier.
    pub id: FrameLayoutId,
    /// The owning MIR function.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The ordered logical slots in layout order.
    pub slots: Vec<FrameSlot>,
    /// The SSA value slot range in layout order.
    pub value_slots: std::ops::Range<u32>,
    /// The mutable local slot range in layout order.
    pub local_slots: std::ops::Range<u32>,
    /// The function environment slot when present.
    pub environment_slot: Option<u32>,
}

impl FrameLayout {
    /// Return the slot at the given layout index.
    pub fn slot(&self, slot: u32) -> Option<&FrameSlot> {
        self.slots.get(slot as usize)
    }
}

/// One captured frame-local stack allocation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AllocationImage {
    /// The raw byte storage for this allocation.
    pub bytes: Vec<u8>,
    /// The stored raw storage type.
    pub storage_type: mir::LocalNodeId<mir::Type>,
}

/// One durable logical frame image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameImage {
    /// The frame layout used by this activation.
    pub frame_layout: FrameLayoutId,
    /// The current resume point for this activation.
    pub resume_point: ResumePointId,
    /// The pending transfer owned by this frame when another frame is active.
    pub transfer: Option<ControlTransfer>,
    /// The logical slot payloads in layout order.
    pub slots: Vec<MaterializedValue>,
    /// The captured frame-local stack allocations.
    pub allocations: Vec<Option<AllocationImage>>,
}
