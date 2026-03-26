use destack_mir as mir;

use crate::{FrameTransfer, FrameValue, ResumePointId};

/// The identifier for one frame layout table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameLayoutId(pub u32);

/// The semantic role of one logical frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameSlotKind {
    /// One SSA value slot.
    Value,
    /// One mutable local slot.
    Local,
    /// The function environment slot.
    Environment,
}

/// The originating MIR entity represented by one frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameSlotSource {
    /// One SSA value slot.
    Value(mir::Value),
    /// One mutable local slot.
    Local(mir::LocalNodeId<mir::Local>),
    /// The function environment slot.
    Environment,
}

/// The runtime payload class stored in one frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameSlotValueClass {
    /// One plain scalar or non-reference payload.
    Plain,
    /// One managed heap reference.
    ManagedReference,
    /// One managed object handle.
    ManagedObject,
    /// One raw heap pointer.
    RawPointer,
    /// One shared byte-space pointer.
    SharedPointer,
    /// One stack pointer.
    StackPointer,
    /// One local pointer.
    LocalPointer,
    /// One global pointer.
    GlobalPointer,
    /// One MIR function reference.
    Function,
    /// One pointer-like value with unresolved storage class.
    UnknownPointer,
}

impl FrameSlotValueClass {
    /// Return whether this slot class may hold one managed GC root.
    pub fn contains_managed_references(self) -> bool {
        matches!(self, Self::ManagedReference | Self::ManagedObject)
    }
}

/// One logical slot in one frame layout.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameSlot {
    /// The semantic role of the slot.
    pub kind: FrameSlotKind,
    /// The originating MIR entity represented by this slot.
    pub source: FrameSlotSource,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
    /// The runtime payload class stored in this slot.
    pub value_class: FrameSlotValueClass,
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

    /// Return whether the given layout slot may hold one managed root.
    pub fn contains_managed_references(&self, slot: u32) -> bool {
        self.slot(slot)
            .map(|slot| slot.value_class.contains_managed_references())
            .unwrap_or(false)
    }
}

/// One durable logical frame image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameImage {
    /// The frame layout used by this activation.
    pub frame_layout: FrameLayoutId,
    /// The current resume point for this activation.
    pub resume_point: ResumePointId,
    /// The pending transfer owned by this frame when another frame is active.
    pub transfer: Option<FrameTransfer>,
    /// The logical slot payloads in layout order.
    pub slots: Vec<FrameValue>,
}
