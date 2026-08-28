use destack_core::{
    EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionImageError,
    SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Entry, FrameMap, FrameMapBuilder, FrameSlot};

const ABI_VERSION: u32 = 1;

/// Durable WebAssembly code produced for one Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// Destack WebAssembly ABI version required by this module.
    pub abi_version: u32,
    /// Encoded linked WebAssembly module bytes.
    module: SectionSlice<u8>,
    /// Exported entries keyed by Program function id.
    entries: SectionSlice<Optional<Entry>>,
    /// Physical frame maps keyed by Program frame state id.
    frames: SectionSlice<Optional<FrameMap>>,
    /// Flattened physical frame slots.
    slots: SectionSlice<FrameSlot>,
}

impl Code {
    /// Validate every flattened WebAssembly frame range.
    pub fn validate(&self, sections: SectionImage<'_>) -> Result<(), SectionImageError> {
        let frame_maps = sections.entries(self.frames);

        // validate flattened frame slots
        let slots = sections.entries(self.slots);
        for frame in frame_maps.iter().filter_map(|frame| frame.get()) {
            frame.slots.validate(slots.len())?;
        }

        Ok(())
    }

    /// Return one exported WebAssembly entry.
    pub fn entry(&self, sections: SectionImage<'_>, function: usize) -> Option<Entry> {
        sections
            .entries(self.entries)
            .get(function)
            .and_then(|entry| entry.get())
    }

    /// Return one physical WebAssembly frame map.
    pub fn frame(&self, sections: SectionImage<'_>, state: usize) -> Option<FrameMap> {
        sections
            .entries(self.frames)
            .get(state)
            .and_then(|frame| frame.get())
    }

    /// Return physical slots in one WebAssembly frame map.
    pub fn slots<'a>(&self, sections: SectionImage<'a>, frame: FrameMap) -> &'a [FrameSlot] {
        frame.slots.slice(sections.entries(self.slots))
    }

    /// Return the encoded linked WebAssembly module.
    pub fn module<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.module)
    }
}

/// Mutable WebAssembly code before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBuilder {
    /// Encoded linked WebAssembly module bytes.
    module: Vec<u8>,
    /// Exported entries keyed by Program function id.
    entries: Vec<Option<Entry>>,
    /// Physical frame maps keyed by Program frame state id.
    frames: Vec<Option<FrameMapBuilder>>,
}

impl CodeBuilder {
    /// Create one WebAssembly code builder.
    pub fn new(module: impl Into<Vec<u8>>) -> Self {
        Self {
            module: module.into(),
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
            module: sections.insert(self.module),
            entries: sections.insert(entries),
            frames: sections.insert(frames),
            slots: sections.insert(slots.into_entries()),
        }
    }
}
