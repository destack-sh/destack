use serde::{Deserialize, Serialize};

use crate::{CodeOffset, EntryFn};

/// One native entry id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EntryId(pub u32);

/// One native entry symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrySymbol {
    /// The entry id.
    pub id: EntryId,
    /// The runtime entry name.
    pub name: String,
    /// The code offset for this entry.
    pub offset: CodeOffset,
}

/// One loaded native entry.
#[derive(Clone)]
pub struct Entry {
    /// The entry id.
    pub id: EntryId,
    /// The loaded entry function pointer.
    pub function: EntryFn,
}

impl Entry {
    /// Load one entry symbol.
    pub fn load(symbol: EntrySymbol, function: EntryFn) -> Self {
        Self {
            id: symbol.id,
            function,
        }
    }
}

impl std::fmt::Debug for Entry {
    /// Format this entry without exposing a raw function pointer.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Entry")
            .field("id", &self.id)
            .field("is_loaded", &true)
            .finish()
    }
}
