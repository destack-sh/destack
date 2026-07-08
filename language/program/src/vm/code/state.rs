use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameSlotId, FrameStateId, ProgramPoint};

use super::MoveSlot;

/// VM resume states keyed by execution frame state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResumeTable {
    /// Resume states by dense frame state id.
    states: SectionSlice<ResumeState>,
    /// Resume state ids sorted by program point.
    points: SectionSlice<ResumePoint>,
    /// Flattened frame entry bindings.
    bindings: SectionSlice<FrameBinding>,
}

impl ResumeTable {
    /// Build a section-backed resume table.
    pub fn pack(sections: &mut SectionPacker, states: Vec<ResumeStateBuilder>) -> Self {
        let mut state_entries = Vec::with_capacity(states.len());
        let mut point_entries = Vec::with_capacity(states.len());
        let mut bindings = EntryStore::new();

        // flatten variable frame-entry payloads
        for (index, state) in states.into_iter().enumerate() {
            let frame_state = FrameStateId(index as u32);
            let entry = state.entry.map(|entry| FrameEntry {
                bindings: bindings.append(entry.bindings),
                received_value: entry.received_value.into(),
            });

            state_entries.push(ResumeState {
                point: state.point,
                source_point: state.source_point.into(),
                entry: entry.into(),
                return_destination: state.return_destination.into(),
            });
            point_entries.push(ResumePoint {
                point: state.point,
                frame_state,
            });
        }

        // sort point index for binary search lookup
        point_entries.sort_unstable_by_key(|entry| entry.point);

        Self {
            states: sections.insert(state_entries),
            points: sections.insert(point_entries),
            bindings: sections.insert(bindings.into_entries()),
        }
    }

    /// Return the number of resume states.
    pub fn len(&self, sections: SectionImage<'_>) -> usize {
        sections.entries(self.states).len()
    }

    /// Return whether there are no resume states.
    pub fn is_empty(&self, sections: SectionImage<'_>) -> bool {
        self.len(sections) == 0
    }

    /// Return one resume state.
    pub fn state(&self, sections: SectionImage<'_>, id: FrameStateId) -> Option<ResumeState> {
        sections.entries(self.states).get(id.0 as usize).copied()
    }

    /// Return one frame entry.
    pub fn entry<'a>(
        &self,
        sections: SectionImage<'a>,
        state: ResumeState,
    ) -> Option<FrameEntryCode<'a>> {
        let entry = state.entry.get()?;

        Some(FrameEntryCode {
            entry,
            bindings: sections.range(self.bindings, entry.bindings),
        })
    }

    /// Return one resume state id by program point.
    pub fn state_id_at(
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

/// Build-time VM state for one resumable frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ResumeStateBuilder {
    /// The program point where this state resumes.
    pub point: ProgramPoint,
    /// The source instruction point within the lowered block, when one exists.
    pub source_point: Option<u32>,
    /// Entry bindings for block-entry states.
    pub entry: Option<FrameEntryBuilder>,
    /// Caller return destination for post-call states.
    pub return_destination: Option<MoveSlot>,
}

/// VM state for one resumable frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResumeState {
    /// The program point where this state resumes.
    pub point: ProgramPoint,
    /// The source instruction point within the lowered block, when one exists.
    pub source_point: Optional<u32>,
    /// Entry bindings for block-entry states.
    pub entry: Optional<FrameEntry>,
    /// Caller return destination for post-call states.
    pub return_destination: Optional<MoveSlot>,
}

/// Build-time VM entry bindings for one resume state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameEntryBuilder {
    /// The slot bindings applied on entry.
    pub bindings: Vec<FrameBinding>,
    /// The implicit received value slot.
    pub received_value: Option<FrameSlotId>,
}

/// VM entry binding entry for one resume state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameEntry {
    /// The slot bindings applied on entry.
    pub bindings: EntryRange<FrameBinding>,
    /// The implicit received value slot.
    pub received_value: Optional<FrameSlotId>,
}

/// Borrowed VM entry bindings for one resume state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameEntryCode<'a> {
    /// The frame entry entry.
    pub entry: FrameEntry,
    /// The slot bindings applied on entry.
    pub bindings: &'a [FrameBinding],
}

impl FrameEntryCode<'_> {
    /// Return the implicit received value slot.
    pub fn received_value(self) -> Option<FrameSlotId> {
        self.entry.received_value.get()
    }
}

/// One frame slot binding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameBinding {
    /// The source frame slot.
    pub source: FrameSlotId,
    /// The destination frame slot.
    pub destination: FrameSlotId,
}

/// Dense resume state lookup entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResumePoint {
    /// The program point for this resume state.
    pub point: ProgramPoint,
    /// The resume state at this point.
    pub frame_state: FrameStateId,
}

// SAFETY: VM resume entries are repr(C), Copy, and contain only section entries.
unsafe impl SectionEntry for ResumeState {}
unsafe impl SectionEntry for FrameEntry {}
unsafe impl SectionEntry for FrameBinding {}
unsafe impl SectionEntry for ResumePoint {}
