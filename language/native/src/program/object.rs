use std::collections::HashMap;
use std::collections::hash_map::Entry as HashEntry;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use destack_engine::{ProgramLayout, StaticSpace};
use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::{EntryId, EntrySymbol, Text};

/// Native object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    /// The code bytes.
    pub text: Text,
    /// The program static memory.
    pub static_space: StaticSpace,
    /// Runtime layout tables for this program.
    pub layout: ProgramLayout,
    /// Heap trace table for managed allocation metadata.
    pub trace_table: Arc<mir::TraceTable>,
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
        layout: ProgramLayout,
        trace_table: Arc<mir::TraceTable>,
        entries: Vec<EntrySymbol>,
    ) -> Result<Self, ObjectError> {
        let mut entry_by_name = HashMap::new();
        for (index, entry) in entries.iter().enumerate() {
            // entries are indexed directly by id
            let expected = EntryId(index as u32);
            if entry.id != expected {
                return Err(ObjectError::EntryIdMismatch {
                    expected,
                    actual: entry.id,
                });
            }

            // runtime names must resolve to one entry
            match entry_by_name.entry(entry.name.clone()) {
                HashEntry::Occupied(_) => {
                    return Err(ObjectError::DuplicateEntryName {
                        name: entry.name.clone(),
                    });
                }
                HashEntry::Vacant(slot) => {
                    slot.insert(entry.id);
                }
            }
        }

        Ok(Self {
            text,
            static_space,
            layout,
            trace_table,
            entries,
            entry_by_name,
        })
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

/// Native object construction error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectError {
    /// An entry id does not match its dense index.
    EntryIdMismatch {
        /// The expected entry id.
        expected: EntryId,
        /// The actual entry id.
        actual: EntryId,
    },
    /// Two entries use the same runtime name.
    DuplicateEntryName {
        /// The duplicated entry name.
        name: String,
    },
}

impl fmt::Display for ObjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryIdMismatch { expected, actual } => write!(
                formatter,
                "native entry id {actual:?} does not match dense index {expected:?}"
            ),
            Self::DuplicateEntryName { name } => {
                write!(formatter, "duplicate native entry name {name}")
            }
        }
    }
}

impl Error for ObjectError {}
