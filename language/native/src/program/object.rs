use std::collections::HashMap;

use crate::{EntryId, EntryImage, Text};

/// Emitted native object plus managed runtime metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Object {
    /// The emitted object code.
    pub text: Text,
    /// The immutable program static memory.
    pub static_space: destack_engine::StaticSpace,
    /// The metadata needed by GC, safepoints, yielding, and images.
    pub metadata: destack_engine::ProgramMetadata,
    /// The named entry table.
    entries: Vec<EntryImage>,
    /// Entry id by runtime entry name.
    entry_by_name: HashMap<String, EntryId>,
}

impl Object {
    /// Create one emitted native object.
    pub fn new(
        text: Text,
        static_space: destack_engine::StaticSpace,
        metadata: destack_engine::ProgramMetadata,
        entries: Vec<EntryImage>,
    ) -> Self {
        let entry_by_name = entries
            .iter()
            .map(|entry| (entry.name.clone(), entry.id))
            .collect();

        Self {
            text,
            static_space,
            metadata,
            entries,
            entry_by_name,
        }
    }

    /// Return one entry by id.
    pub fn entry(&self, id: EntryId) -> Option<&EntryImage> {
        self.entries.get(id.0 as usize)
    }

    /// Return one entry by runtime name.
    pub fn entry_by_name(&self, name: &str) -> Option<&EntryImage> {
        let id = self.entry_by_name.get(name)?;

        self.entry(*id)
    }

    /// Return all entries in entry id order.
    pub fn entries(&self) -> &[EntryImage] {
        &self.entries
    }
}
