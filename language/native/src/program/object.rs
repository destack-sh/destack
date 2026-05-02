use std::collections::HashMap;

use destack_engine::{Metadata, StaticSpace};
use serde::{Deserialize, Serialize};

use crate::{EntryId, EntrySymbol, Text};

/// Native object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    /// The code bytes.
    pub text: Text,
    /// The program static memory.
    pub static_space: StaticSpace,
    /// The program execution metadata.
    pub metadata: Metadata,
    /// The entries by id.
    entries: Vec<EntrySymbol>,
    /// Entry id by runtime entry name.
    entry_by_name: HashMap<String, EntryId>,
}

impl Object {
    /// Create one native object.
    pub fn new(
        text: Text,
        static_space: StaticSpace,
        metadata: Metadata,
        entries: Vec<EntrySymbol>,
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
    pub fn entry(&self, id: EntryId) -> Option<&EntrySymbol> {
        self.entries.get(id.0 as usize)
    }

    /// Return one entry by runtime name.
    pub fn entry_by_name(&self, name: &str) -> Option<&EntrySymbol> {
        let id = self.entry_by_name.get(name)?;

        self.entry(*id)
    }

    /// Return all entries in entry id order.
    pub fn entries(&self) -> &[EntrySymbol] {
        &self.entries
    }
}
