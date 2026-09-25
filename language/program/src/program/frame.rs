use std::mem::size_of;

use serde::{Deserialize, Serialize};
use tspp_core::{EntryRange, EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use tspp_serde::Reflect;

use super::{FramePoint, TypeId};

/// Program frame states and their canonical layouts.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FrameTable {
    /// Frame states keyed by dense frame state id.
    states: SectionSlice<FrameState>,
    /// Canonical frame layouts keyed by dense frame layout id.
    layouts: SectionSlice<FrameLayout>,
    /// Flattened frame slots.
    slots: SectionSlice<FrameSlot>,
}

impl FrameTable {
    /// Return one frame state by id.
    pub fn state<'a>(
        &self,
        sections: SectionImage<'a>,
        state: FrameStateId,
    ) -> Option<&'a FrameState> {
        self.states(sections).get(state.index())
    }

    /// Return the frame state at one logical frame coordinate.
    pub fn state_at(&self, sections: SectionImage<'_>, point: FramePoint) -> Option<FrameStateId> {
        let states = self.states(sections);
        let index = states
            .binary_search_by_key(&point, |state| state.point)
            .ok()?;

        Some(FrameStateId(index as u32))
    }

    /// Return all frame states in logical coordinate order.
    pub fn states<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameState] {
        sections.entries(self.states)
    }

    /// Return one canonical frame layout by id.
    pub fn layout<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: FrameLayoutId,
    ) -> Option<&'a FrameLayout> {
        self.layouts(sections).get(layout.index())
    }

    /// Return all canonical frame layouts.
    pub fn layouts<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameLayout] {
        sections.entries(self.layouts)
    }

    /// Return the slots in one canonical frame layout.
    pub fn slots<'a>(&self, sections: SectionImage<'a>, layout: &FrameLayout) -> &'a [FrameSlot] {
        layout.slots.slice(sections.entries(self.slots))
    }

    /// Build one frame table into Program sections.
    pub(crate) fn pack(builder: FrameTableBuilder, sections: &mut SectionBuilder) -> Self {
        let mut slots = EntryStore::new();
        let layouts = builder
            .layouts
            .into_iter()
            .map(|layout| layout.build(&mut slots))
            .collect::<Vec<_>>();

        Self {
            states: sections.insert(builder.states),
            layouts: sections.insert(layouts),
            slots: sections.insert(slots.into_entries()),
        }
    }

    /// Return whether every frame layout range fits the slot column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let slots = sections.entries(self.slots).len();

        sections
            .entries(self.layouts)
            .iter()
            .all(|layout| layout.slots.fits(slots))
    }
}

/// Mutable Program frame table before section packing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameTableBuilder {
    /// Frame states in logical coordinate order.
    states: Vec<FrameState>,
    /// Canonical frame layouts in frame layout id order.
    layouts: Vec<FrameLayoutBuilder>,
}

impl FrameTableBuilder {
    /// Create one empty frame table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set frame states in logical coordinate order.
    pub fn states(mut self, states: impl IntoIterator<Item = FrameState>) -> Self {
        self.states = states.into_iter().collect();

        self
    }

    /// Set canonical frame layouts in frame layout id order.
    pub fn layouts(mut self, layouts: impl IntoIterator<Item = FrameLayoutBuilder>) -> Self {
        self.layouts = layouts.into_iter().collect();

        self
    }
}

/// Canonical state at one logical frame coordinate.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameState {
    /// The logical frame coordinate represented by this state.
    pub point: FramePoint,
    /// The exact canonical layout of the live frame values.
    pub layout: FrameLayoutId,
}

impl FrameState {
    /// Create one canonical frame state.
    pub const fn new(point: FramePoint, layout: FrameLayoutId) -> Self {
        Self { point, layout }
    }
}

/// Canonical packed layout of the values live in one frame state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameLayout {
    /// Live frame slots in acquisition order.
    pub slots: EntryRange<FrameSlot>,
    /// Complete packed frame byte length.
    pub byte_len: u32,
    /// Required frame byte alignment.
    pub alignment: u32,
}

impl FrameLayout {
    /// Return the complete packed frame byte length.
    pub const fn byte_len(self) -> u32 {
        self.byte_len
    }

    /// Return the required frame byte alignment.
    pub const fn alignment(self) -> u32 {
        self.alignment
    }
}

/// Mutable canonical frame layout before section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameLayoutBuilder {
    /// Live frame slots in acquisition order.
    slots: Vec<FrameSlot>,
    /// Complete packed frame byte length.
    byte_len: u32,
    /// Required frame byte alignment.
    alignment: u32,
}

impl FrameLayoutBuilder {
    /// Create one empty canonical frame layout.
    pub fn new(byte_len: u32, alignment: u32) -> Self {
        Self {
            slots: Vec::new(),
            byte_len,
            alignment,
        }
    }

    /// Set live frame slots in acquisition order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }

    /// Build this layout into the flattened frame slot store.
    fn build(self, slots: &mut EntryStore<FrameSlot>) -> FrameLayout {
        FrameLayout {
            slots: slots.append(self.slots),
            byte_len: self.byte_len,
            alignment: self.alignment,
        }
    }
}

/// One typed value range inside a canonical frame layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameSlot {
    /// The byte offset from the canonical frame base.
    pub offset: u32,
    /// The stored value byte length.
    pub byte_len: u32,
    /// The stored value type.
    pub ty: TypeId,
}

impl FrameSlot {
    /// Create one typed canonical frame slot.
    pub const fn new(offset: u32, byte_len: u32, ty: TypeId) -> Self {
        Self {
            offset,
            byte_len,
            ty,
        }
    }
}

/// Dense identity of one canonical frame state.
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

impl FrameStateId {
    /// Return this id as a dense frame state index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Dense identity of one canonical frame layout.
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

impl FrameLayoutId {
    /// Return this id as a dense frame layout index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<FrameState>() == 16);
const _: () = assert!(size_of::<FrameLayout>() == 16);
const _: () = assert!(size_of::<FrameSlot>() == 12);
