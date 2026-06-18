use serde::{Deserialize, Serialize};

use crate::{FrameStateId, StorageLayoutId};

/// Materialized frame captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedFrame {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}

/// One frame layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameLayoutId(pub u32);

/// One frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameSlotId(pub u32);

/// Physical storage slot inside one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSlot {
    /// The slot id.
    pub id: FrameSlotId,
    /// The byte offset from the frame base.
    pub offset: u32,
    /// The slot byte length.
    pub byte_len: u32,
    /// The slot byte alignment.
    pub alignment: u16,
    /// Whether this slot stores one cell.
    pub is_cell: bool,
    /// The slot storage layout.
    pub layout: StorageLayoutId,
}

/// Physical byte layout for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameLayout {
    /// The layout id.
    pub id: FrameLayoutId,
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
        self.value(value).map(|slot| slot.id)
    }

    /// Return the frame slot id for one local.
    pub fn local_slot_id(&self, local: u32) -> Option<FrameSlotId> {
        self.local(local).map(|slot| slot.id)
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
        (0..self.slot_len()).map(|index| FrameSlotId(index as u32))
    }
}
