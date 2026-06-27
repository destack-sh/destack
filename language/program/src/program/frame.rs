use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::TypeId;

/// Logical executable frame state id at a resumable program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FrameStateId(pub u32);

/// One physical executable frame layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FrameLayoutId(pub u32);

/// One physical executable frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FrameSlotId(pub u32);

impl From<u32> for FrameStateId {
    /// Convert one raw frame state id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<FrameStateId> for u32 {
    /// Convert one frame state id into its raw value.
    fn from(id: FrameStateId) -> Self {
        id.0
    }
}

impl From<u32> for FrameLayoutId {
    /// Convert one raw frame layout id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<FrameLayoutId> for u32 {
    /// Convert one frame layout id into its raw value.
    fn from(id: FrameLayoutId) -> Self {
        id.0
    }
}

impl From<u32> for FrameSlotId {
    /// Convert one raw frame slot id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<FrameSlotId> for u32 {
    /// Convert one frame slot id into its raw value.
    fn from(id: FrameSlotId) -> Self {
        id.0
    }
}

/// Physical execution frame layout and materialization tables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameTable {
    /// Frame materializations by frame state id.
    pub materializations: Vec<FrameMaterialization>,
    /// Frame layouts by id.
    pub layouts: Vec<FrameLayout>,
}

impl FrameTable {
    /// Return one frame layout by id.
    pub fn layout(&self, layout: FrameLayoutId) -> Option<&FrameLayout> {
        self.layouts.get(layout.0 as usize)
    }

    /// Return one frame materialization by state id.
    pub fn materialization(&self, state: FrameStateId) -> Option<&FrameMaterialization> {
        self.materializations.get(state.0 as usize)
    }
}

/// Physical storage slot inside one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameSlot {
    /// The byte offset from the frame base.
    pub offset: u32,
    /// The slot byte length.
    pub byte_len: u32,
    /// The slot byte alignment.
    pub alignment: u16,
    /// The slot value type.
    pub ty: TypeId,
}

/// Physical byte layout for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameLayout {
    /// Slots in frame order.
    pub slots: Vec<FrameSlot>,
    /// Number of SSA value slots.
    pub value_count: u32,
    /// Number of local slots.
    pub local_count: u32,
    /// Callable environment slot.
    pub environment_slot: Option<FrameSlotId>,
    /// The frame byte length.
    pub byte_len: u32,
}

impl FrameLayout {
    /// Return one frame slot by id.
    pub fn slot(&self, id: FrameSlotId) -> Option<&FrameSlot> {
        self.slots.get(id.0 as usize)
    }

    /// Return the frame slot id for one SSA value.
    pub fn value_slot_id(&self, value: u32) -> Option<FrameSlotId> {
        (value < self.value_count).then_some(value.into())
    }

    /// Return the frame slot id for one local.
    pub fn local_slot_id(&self, local: u32) -> Option<FrameSlotId> {
        (local < self.local_count).then_some((self.value_count + local).into())
    }

    /// Return the value slot at one SSA value index.
    pub fn value(&self, value: u32) -> Option<&FrameSlot> {
        if value < self.value_count {
            self.slots.get(value as usize)
        } else {
            None
        }
    }

    /// Return the local slot at the given local index.
    pub fn local(&self, local: u32) -> Option<&FrameSlot> {
        if local >= self.local_count {
            return None;
        }

        let index = self.value_count + local;

        self.slots.get(index as usize)
    }

    /// Return the SSA value addressed by one slot id.
    pub fn value_for_slot(&self, id: FrameSlotId) -> Option<u32> {
        if id.0 < self.value_count {
            Some(id.0)
        } else {
            None
        }
    }

    /// Return the local addressed by one slot id.
    pub fn local_for_slot(&self, id: FrameSlotId) -> Option<u32> {
        if id.0 < self.value_count {
            return None;
        }

        let local_index = id.0 - self.value_count;
        if local_index < self.local_count {
            Some(local_index)
        } else {
            None
        }
    }

    /// Return whether one slot id addresses the callable environment.
    pub fn is_environment_slot(&self, id: FrameSlotId) -> bool {
        self.environment_slot == Some(id)
    }

    /// Return the callable environment slot when present.
    pub fn environment(&self) -> Option<&FrameSlot> {
        self.environment_slot.and_then(|id| self.slot(id))
    }

    /// Return all value slots.
    pub fn values(&self) -> &[FrameSlot] {
        &self.slots[..self.value_count as usize]
    }

    /// Return all local slots.
    pub fn locals(&self) -> &[FrameSlot] {
        let start = self.value_count as usize;
        let end = start + self.local_count as usize;

        &self.slots[start..end]
    }

    /// Return the frame slot count.
    pub fn slot_len(&self) -> usize {
        self.slots.len()
    }

    /// Return all frame slot ids.
    pub fn slot_ids(&self) -> impl Iterator<Item = FrameSlotId> {
        (0..self.slot_len()).map(|index| (index as u32).into())
    }
}

/// Plan for reconstructing one execution frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameMaterialization {
    /// The reconstructed frame layout.
    pub frame_layout: FrameLayoutId,
    /// Frame slots copied into the materialized frame.
    pub copied_slots: Vec<FrameSlotId>,
}

impl FrameMaterialization {
    /// Return each source frame slot once.
    pub fn copied_slots(&self) -> impl Iterator<Item = FrameSlotId> + '_ {
        self.copied_slots.iter().copied()
    }
}
