use destack_core::{
    EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use super::{Entry, FrameMap, FrameMapBuilder, FrameSlot};

const ABI_VERSION: u32 = 1;

/// Durable WebAssembly code produced for one Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// Destack WebAssembly ABI version required by this module.
    pub abi_version: u32,
    /// The linked WebAssembly module.
    pub module: ContentId,
    /// Exported entries keyed by Program function id.
    entries: SectionSlice<Optional<Entry>>,
    /// Physical frame maps keyed by Program frame state id.
    frames: SectionSlice<Optional<FrameMap>>,
    /// Flattened physical frame slots.
    slots: SectionSlice<FrameSlot>,
}

impl Code {
    /// Return one exported WebAssembly entry.
    pub fn entry(self, sections: SectionImage<'_>, function: usize) -> Option<Entry> {
        sections
            .entries(self.entries)
            .get(function)
            .and_then(|entry| entry.get())
    }

    /// Return one physical WebAssembly frame map.
    pub fn frame(self, sections: SectionImage<'_>, state: usize) -> Option<FrameMap> {
        sections
            .entries(self.frames)
            .get(state)
            .and_then(|frame| frame.get())
    }

    /// Return physical slots in one WebAssembly frame map.
    pub fn slots<'a>(self, sections: SectionImage<'a>, frame: FrameMap) -> &'a [FrameSlot] {
        frame.slots.slice(sections.entries(self.slots))
    }

    /// Return all content ids referenced by this WebAssembly code.
    pub fn content_ids(self) -> Vec<ContentId> {
        vec![self.module]
    }
}

/// Mutable WebAssembly code before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBuilder {
    /// The linked WebAssembly module.
    module: ContentId,
    /// Exported entries keyed by Program function id.
    entries: Vec<Option<Entry>>,
    /// Physical frame maps keyed by Program frame state id.
    frames: Vec<Option<FrameMapBuilder>>,
}

impl CodeBuilder {
    /// Create one WebAssembly code builder.
    pub fn new(module: ContentId) -> Self {
        Self {
            module,
            entries: Vec::new(),
            frames: Vec::new(),
        }
    }

    /// Set exported entries in dense Program function order.
    pub fn entries(mut self, entries: impl IntoIterator<Item = Option<Entry>>) -> Self {
        self.entries = entries.into_iter().collect();

        self
    }

    /// Set physical frame maps in dense Program frame state order.
    pub fn frames(mut self, frames: impl IntoIterator<Item = Option<FrameMapBuilder>>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Build this WebAssembly code into Program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        let mut slots = EntryStore::new();
        let frames = self
            .frames
            .into_iter()
            .map(|frame| frame.map(|frame| frame.build(&mut slots)))
            .map(Optional::from)
            .collect::<Vec<_>>();
        let entries = self
            .entries
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();

        Code {
            abi_version: ABI_VERSION,
            module: self.module,
            entries: sections.insert(entries),
            frames: sections.insert(frames),
            slots: sections.insert(slots.into_entries()),
        }
    }
}
