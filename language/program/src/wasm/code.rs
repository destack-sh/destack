use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

use crate::{FrameStateId, FunctionId};

const WASM_ABI_VERSION: u32 = 1;

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
    pub fn entry(self, sections: SectionImage<'_>, function: FunctionId) -> Option<Entry> {
        sections
            .entries(self.entries)
            .get(function.index())
            .and_then(|entry| entry.get())
    }

    /// Return one physical WebAssembly frame map.
    pub fn frame(self, sections: SectionImage<'_>, state: FrameStateId) -> Option<FrameMap> {
        sections
            .entries(self.frames)
            .get(state.index())
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
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> Code {
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
            abi_version: WASM_ABI_VERSION,
            module: self.module,
            entries: sections.insert(entries),
            frames: sections.insert(frames),
            slots: sections.insert(slots.into_entries()),
        }
    }
}

/// One WebAssembly export implementing a Program function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Entry {
    /// The generated WebAssembly export name.
    pub export: StringId,
}

impl Entry {
    /// Create one exported WebAssembly function entry.
    pub const fn new(export: StringId) -> Self {
        Self { export }
    }
}

/// Physical WebAssembly projection of one canonical frame state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameMap {
    /// Materialized slots in canonical Program frame slot order.
    slots: EntryRange<FrameSlot>,
}

/// Mutable WebAssembly frame map before Program section packing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameMapBuilder {
    /// Materialized slots in canonical Program frame slot order.
    slots: Vec<FrameSlot>,
}

impl FrameMapBuilder {
    /// Create one empty WebAssembly frame map builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set materialized slots in canonical Program frame slot order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }

    /// Build this frame map into flattened slot storage.
    fn build(self, slots: &mut EntryStore<FrameSlot>) -> FrameMap {
        FrameMap {
            slots: slots.append(self.slots),
        }
    }
}

/// One canonical value materialized in WebAssembly activation memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameSlot {
    /// Byte offset from the WebAssembly activation frame base.
    pub offset: u32,
}

impl FrameSlot {
    /// Create one materialized WebAssembly frame slot.
    pub const fn new(offset: u32) -> Self {
        Self { offset }
    }
}
