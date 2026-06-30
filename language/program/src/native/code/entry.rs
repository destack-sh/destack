use destack_core::{Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameStateId, FunctionId};

/// Native entry table keyed by program ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EntryTable {
    /// Native function entries keyed by program function id.
    pub(super) function: SectionSlice<Optional<Entry>>,
    /// Native resume entries keyed by frame state id.
    pub(super) resume: SectionSlice<Optional<Resume>>,
}

impl EntryTable {
    /// Pack one native entry table.
    pub fn pack(
        sections: &mut SectionPacker,
        function: Vec<Option<Entry>>,
        resume: Vec<Option<Resume>>,
    ) -> Self {
        let function = function.into_iter().map(Optional::from).collect::<Vec<_>>();
        let resume = resume.into_iter().map(Optional::from).collect::<Vec<_>>();

        Self {
            function: sections.insert(function),
            resume: sections.insert(resume),
        }
    }

    /// Return one native function entry.
    pub fn function(&self, sections: SectionImage<'_>, function: FunctionId) -> Option<Entry> {
        sections
            .entries(self.function)
            .get(function.index())
            .and_then(|entry| entry.get())
    }

    /// Return one native resume entry.
    pub fn resume(&self, sections: SectionImage<'_>, frame_state: FrameStateId) -> Option<Resume> {
        sections
            .entries(self.resume)
            .get(frame_state.0 as usize)
            .and_then(|entry| entry.get())
    }

    /// Return native function entries in dense program function id order.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Entry>] {
        sections.entries(self.function)
    }

    /// Return native resume entries in dense frame state id order.
    pub fn resumes<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Resume>] {
        sections.entries(self.resume)
    }
}

/// Native function entry resolved by symbol name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Entry {
    /// The function implemented by this entry.
    pub function: FunctionId,
    /// The native symbol exported by the linked image.
    pub symbol: StringId,
}

impl Entry {
    /// Create one native function entry.
    pub fn new(function: FunctionId, symbol: StringId) -> Self {
        Self { function, symbol }
    }
}

/// Native continuation resume entry resolved by symbol name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Resume {
    /// The frame state resumed by this entry.
    pub frame_state: FrameStateId,
    /// The native symbol exported by the linked image.
    pub symbol: StringId,
}

// SAFETY: native entries are fixed-width program entries.
unsafe impl SectionEntry for Entry {}

// SAFETY: native resume entries are fixed-width program entries.
unsafe impl SectionEntry for Resume {}

impl Resume {
    /// Create one native resume entry.
    pub fn new(frame_state: FrameStateId, symbol: StringId) -> Self {
        Self {
            frame_state,
            symbol,
        }
    }
}
