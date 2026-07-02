use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::TypeId;

/// Logical frame state id at a resumable program point.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct FrameStateId(pub u32);

/// One physical frame layout.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct FrameLayoutId(pub u32);

/// One physical frame slot.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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

/// Durable frame image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameImage {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The caller return frame state.
    pub return_state: Option<FrameStateId>,
    /// The byte offset inside the captured stack image.
    pub stack_offset: usize,
    /// The captured frame byte width.
    pub byte_len: usize,
}

impl FrameImage {
    /// Return the captured frame byte width.
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

/// Physical execution frame layout and materialization tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameTable {
    /// Frame materializations by frame state id.
    materializations: SectionSlice<FrameMaterialization>,
    /// Frame layouts by id.
    layouts: SectionSlice<FrameLayout>,
    /// Flattened frame slots.
    slots: SectionSlice<FrameSlot>,
    /// Flattened copied materialization slots.
    copied_slots: SectionSlice<FrameSlotId>,
}

impl FrameTable {
    /// Create one frame table from final entry sections.
    pub fn from_entries(
        sections: &mut SectionPacker,
        layouts: &[FrameLayout],
        materializations: &[FrameMaterialization],
        slots: &[FrameSlot],
        copied_slots: &[FrameSlotId],
    ) -> Self {
        let materializations = sections.insert(materializations);
        let layouts = sections.insert(layouts);
        let slots = sections.insert(slots);
        let copied_slots = sections.insert(copied_slots);

        Self {
            materializations,
            layouts,
            slots,
            copied_slots,
        }
    }

    /// Pack one frame table from build-time frame entries.
    pub fn pack(
        sections: &mut SectionPacker,
        layouts: Vec<FrameLayoutBuilder>,
        materializations: Vec<FrameMaterializationBuilder>,
    ) -> Self {
        let mut layout_entries = Vec::with_capacity(layouts.len());
        let mut slots = EntryStore::new();
        let mut materialization_entries = Vec::with_capacity(materializations.len());
        let mut copied_slots = EntryStore::new();

        // flatten frame slot payloads
        for layout in layouts {
            let slot_range = slots.append(layout.slots);

            layout_entries.push(FrameLayout {
                slots: slot_range,
                value_count: layout.value_count,
                local_count: layout.local_count,
                environment_slot: layout.environment_slot.into(),
                byte_len: layout.byte_len,
            });
        }

        // flatten materialization payloads
        for materialization in materializations {
            let copied_slot_range = copied_slots.append(materialization.copied_slots);

            materialization_entries.push(FrameMaterialization {
                frame_layout: materialization.frame_layout,
                copied_slots: copied_slot_range,
            });
        }

        Self::from_entries(
            sections,
            &layout_entries,
            &materialization_entries,
            slots.entries(),
            copied_slots.entries(),
        )
    }

    /// Return one frame layout by id.
    pub fn layout<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: FrameLayoutId,
    ) -> Option<&'a FrameLayout> {
        sections.entries(self.layouts).get(layout.0 as usize)
    }

    /// Return one frame materialization by state id.
    pub fn materialization<'a>(
        &self,
        sections: SectionImage<'a>,
        state: FrameStateId,
    ) -> Option<&'a FrameMaterialization> {
        sections
            .entries(self.materializations)
            .get(state.0 as usize)
    }

    /// Return one frame slot by id.
    pub fn slot<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: &FrameLayout,
        id: FrameSlotId,
    ) -> Option<&'a FrameSlot> {
        self.slots(sections, layout).get(id.0 as usize)
    }

    /// Return all frame slots for one layout.
    pub fn slots<'a>(&self, sections: SectionImage<'a>, layout: &FrameLayout) -> &'a [FrameSlot] {
        layout.slots(sections.entries(self.slots))
    }

    /// Return copied slots for one materialization.
    pub fn copied_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        materialization: &FrameMaterialization,
    ) -> &'a [FrameSlotId] {
        materialization
            .copied_slots
            .slice(sections.entries(self.copied_slots))
    }
}

/// Physical storage slot inside one frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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

impl FrameSlot {
    /// Return the slot byte length.
    pub const fn byte_len(self) -> u32 {
        self.byte_len
    }
}

/// Physical byte layout for one frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameLayout {
    /// Slots in frame order.
    pub slots: EntryRange<FrameSlot>,
    /// Number of SSA value slots.
    pub value_count: u32,
    /// Number of local slots.
    pub local_count: u32,
    /// Callable environment slot.
    pub environment_slot: Optional<FrameSlotId>,
    /// The frame byte length.
    pub byte_len: u32,
}

impl FrameLayout {
    /// Return the frame byte length.
    pub const fn byte_len(self) -> u32 {
        self.byte_len
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
    pub fn value<'a>(&self, slots: &'a [FrameSlot], value: u32) -> Option<&'a FrameSlot> {
        if value < self.value_count {
            slots.get(value as usize)
        } else {
            None
        }
    }

    /// Return the local slot at the given local index.
    pub fn local<'a>(&self, slots: &'a [FrameSlot], local: u32) -> Option<&'a FrameSlot> {
        if local >= self.local_count {
            return None;
        }

        let index = self.value_count + local;

        slots.get(index as usize)
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
        self.environment_slot.get() == Some(id)
    }

    /// Return the callable environment slot when present.
    pub fn environment<'a>(&self, slots: &'a [FrameSlot]) -> Option<&'a FrameSlot> {
        self.environment_slot
            .get()
            .and_then(|id| slots.get(id.0 as usize))
    }

    /// Return all value slots.
    pub fn values<'a>(&self, slots: &'a [FrameSlot]) -> &'a [FrameSlot] {
        &slots[..self.value_count as usize]
    }

    /// Return all local slots.
    pub fn locals<'a>(&self, slots: &'a [FrameSlot]) -> &'a [FrameSlot] {
        let start = self.value_count as usize;
        let end = start + self.local_count as usize;

        &slots[start..end]
    }

    /// Return the frame slot count.
    pub fn slot_len(&self) -> usize {
        self.slots.len as usize
    }

    /// Return all frame slot ids.
    pub fn slot_ids(&self) -> impl Iterator<Item = FrameSlotId> {
        (0..self.slot_len()).map(|index| (index as u32).into())
    }

    /// Return this layout's slots from the global frame slot table.
    pub fn slots<'a>(&self, slots: &'a [FrameSlot]) -> &'a [FrameSlot] {
        self.slots.slice(slots)
    }
}

/// Plan for reconstructing one execution frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameMaterialization {
    /// The reconstructed frame layout.
    pub frame_layout: FrameLayoutId,
    /// Frame slots copied into the materialized frame.
    pub copied_slots: EntryRange<FrameSlotId>,
}

// SAFETY: frame ids, slots, layouts, and materializations are fixed-width entries.
unsafe impl SectionEntry for FrameStateId {}
unsafe impl SectionEntry for FrameLayoutId {}
unsafe impl SectionEntry for FrameSlotId {}
unsafe impl SectionEntry for FrameSlot {}
unsafe impl SectionEntry for FrameLayout {}
unsafe impl SectionEntry for FrameMaterialization {}

impl FrameMaterialization {
    /// Return each source frame slot once.
    pub fn copied_slots<'a>(
        &self,
        slots: &'a [FrameSlotId],
    ) -> impl Iterator<Item = FrameSlotId> + 'a {
        self.copied_slots.slice(slots).iter().copied()
    }
}

/// Build-time physical byte layout for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameLayoutBuilder {
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

/// Build-time plan for reconstructing one execution frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameMaterializationBuilder {
    /// The reconstructed frame layout.
    pub frame_layout: FrameLayoutId,
    /// Frame slots copied into the materialized frame.
    pub copied_slots: Vec<FrameSlotId>,
}
