use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::FunctionId;

/// Native entry table keyed by program ids.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct EntryTable {
    /// Native function entries keyed by program function id.
    pub(super) function: SectionSlice<Optional<Entry>>,
}

/// Build-time native entry table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryTableBuilder {
    /// Function entries in dense program function id order.
    functions: Vec<Option<Entry>>,
}

impl EntryTableBuilder {
    /// Create an empty native entry table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set function entries in dense program function id order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Option<Entry>>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Build this entry table into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> EntryTable {
        let functions = self
            .functions
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();
        EntryTable {
            function: sections.insert(functions),
        }
    }
}

impl EntryTable {
    /// Return one native function entry.
    pub fn function(&self, sections: SectionImage<'_>, function: FunctionId) -> Option<Entry> {
        sections
            .entries(self.function)
            .get(function.index())
            .and_then(|entry| entry.get())
    }

    /// Return native function entries in dense program function id order.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Entry>] {
        sections.entries(self.function)
    }
}

/// Native function entry resolved by symbol name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
