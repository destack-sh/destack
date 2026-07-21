use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{ProgramPoint, TypeId};

/// One executable frame state at a program point.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct FrameStateId(pub u32);

/// One physical frame layout.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct FrameLayoutId(pub u32);

/// One physical frame slot.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
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
    /// The caller normal frame state.
    pub normal_state: Option<FrameStateId>,
    /// The caller unwind frame state.
    pub unwind_state: Option<FrameStateId>,
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

/// Program frame layouts and state tables.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FrameTable {
    /// Frame states by frame state id.
    states: SectionSlice<FrameState>,
    /// Frame layouts by id.
    layouts: SectionSlice<FrameLayout>,
    /// Flattened frame slots.
    slots: SectionSlice<FrameSlot>,
    /// Flattened live frame slots.
    live_slots: SectionSlice<FrameSlotId>,
    /// Frame state ids sorted by program point.
    points: SectionSlice<FramePoint>,
}

impl FrameTable {
    /// Return one frame layout by id.
    pub fn layout<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: FrameLayoutId,
    ) -> Option<&'a FrameLayout> {
        sections.entries(self.layouts).get(layout.0 as usize)
    }

    /// Return one frame state by state id.
    pub fn state<'a>(
        &self,
        sections: SectionImage<'a>,
        state: FrameStateId,
    ) -> Option<&'a FrameState> {
        sections.entries(self.states).get(state.0 as usize)
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

    /// Return live slots for one state.
    pub fn live_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        state: &FrameState,
    ) -> &'a [FrameSlotId] {
        state.live_slots.slice(sections.entries(self.live_slots))
    }

    /// Return one frame state id by program point.
    pub fn state_at(
        &self,
        sections: SectionImage<'_>,
        point: ProgramPoint,
    ) -> Option<FrameStateId> {
        let points = sections.entries(self.points);
        let index = points
            .binary_search_by_key(&point, |entry| entry.point)
            .ok()?;

        Some(points[index].frame_state)
    }
}

/// Build-time frame layouts and states.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTableBuilder {
    /// Physical frame layouts by Program frame layout id.
    layouts: Vec<FrameLayoutBuilder>,
    /// Frame states by Program frame state id.
    states: Vec<FrameStateBuilder>,
}

impl FrameTableBuilder {
    /// Create an empty frame table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical frame layouts.
    pub fn layouts(mut self, layouts: impl IntoIterator<Item = FrameLayoutBuilder>) -> Self {
        self.layouts = layouts.into_iter().collect();

        self
    }

    /// Set resumable frame states.
    pub fn states(mut self, states: impl IntoIterator<Item = FrameStateBuilder>) -> Self {
        self.states = states.into_iter().collect();

        self
    }

    /// Build this frame table into final program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> FrameTable {
        let mut layouts = Vec::with_capacity(self.layouts.len());
        let mut slots = EntryStore::new();
        let mut states = Vec::with_capacity(self.states.len());
        let mut live_slots = EntryStore::new();
        let mut points = Vec::with_capacity(self.states.len());

        // flatten frame slot payloads
        for layout in self.layouts {
            let slot_range = slots.append(layout.slots);

            layouts.push(FrameLayout {
                slots: slot_range,
                environment_slot: layout.environment_slot.into(),
                byte_len: layout.byte_len,
                alignment: layout.alignment,
            });
        }

        // flatten state payloads
        for (index, state) in self.states.into_iter().enumerate() {
            let live_slot_range = live_slots.append(state.live_slots);
            let frame_state = FrameStateId(index as u32);

            states.push(FrameState {
                point: state.point,
                frame_layout: state.frame_layout,
                live_slots: live_slot_range,
            });
            points.push(FramePoint {
                point: state.point,
                frame_state,
            });
        }

        // sort the point index for binary search
        points.sort_unstable_by_key(|entry| entry.point);

        FrameTable {
            states: sections.insert(states),
            layouts: sections.insert(layouts),
            slots: sections.insert(slots.into_entries()),
            live_slots: sections.insert(live_slots.into_entries()),
            points: sections.insert(points),
        }
    }
}

/// Physical storage slot inside one frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameLayout {
    /// Slots in frame order.
    pub slots: EntryRange<FrameSlot>,
    /// Callable environment slot.
    pub environment_slot: Optional<FrameSlotId>,
    /// The frame byte length.
    pub byte_len: u32,
    /// The frame byte alignment.
    pub alignment: u32,
}

impl FrameLayout {
    /// Return the frame byte length.
    pub const fn byte_len(self) -> u32 {
        self.byte_len
    }

    /// Return the frame byte alignment.
    pub const fn alignment(self) -> u32 {
        self.alignment
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

/// Live frame slots at one program point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameState {
    /// The program point represented by this state.
    pub point: ProgramPoint,
    /// The frame layout.
    pub frame_layout: FrameLayoutId,
    /// The live frame slots.
    pub live_slots: EntryRange<FrameSlotId>,
}

/// Dense frame state lookup entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FramePoint {
    /// The indexed program point.
    pub point: ProgramPoint,
    /// The frame state at that point.
    pub frame_state: FrameStateId,
}

impl FrameState {
    /// Return each live frame slot once.
    pub fn live_slots<'a>(
        &self,
        slots: &'a [FrameSlotId],
    ) -> impl Iterator<Item = FrameSlotId> + 'a {
        self.live_slots.slice(slots).iter().copied()
    }
}

/// Build-time physical byte layout for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameLayoutBuilder {
    /// Slots in frame order.
    slots: Vec<FrameSlot>,
    /// Callable environment slot.
    environment_slot: Option<FrameSlotId>,
    /// The frame byte length.
    byte_len: u32,
    /// The frame byte alignment.
    alignment: u32,
}

impl FrameLayoutBuilder {
    /// Create one physical frame layout builder.
    pub fn new(byte_len: u32, alignment: u32) -> Self {
        Self {
            slots: Vec::new(),
            environment_slot: None,
            byte_len,
            alignment,
        }
    }

    /// Set frame slots in physical order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }

    /// Set the callable environment slot.
    pub fn environment_slot(mut self, environment_slot: FrameSlotId) -> Self {
        self.environment_slot = Some(environment_slot);

        self
    }
}

/// Build-time live frame slots at one program point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStateBuilder {
    /// The program point represented by this state.
    point: ProgramPoint,
    /// The frame layout.
    frame_layout: FrameLayoutId,
    /// The live frame slots.
    live_slots: Vec<FrameSlotId>,
}

impl FrameStateBuilder {
    /// Create one live frame state builder.
    pub fn new(point: ProgramPoint, frame_layout: FrameLayoutId) -> Self {
        Self {
            point,
            frame_layout,
            live_slots: Vec::new(),
        }
    }

    /// Set live frame slots.
    pub fn live_slots(mut self, live_slots: impl IntoIterator<Item = FrameSlotId>) -> Self {
        self.live_slots = live_slots.into_iter().collect();

        self
    }
}
