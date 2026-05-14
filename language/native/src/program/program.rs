use std::sync::Arc;

use destack_engine::{ProgramLayout, StaticSpace};

use crate::{CodeMapping, Entry, EntryId, EntrySymbol, Object};

/// Loaded native program.
#[derive(Debug, Clone)]
pub struct Program {
    /// The native object.
    object: Arc<Object>,
    /// The loaded code range.
    code: CodeMapping,
    /// The entries by id.
    entries: Vec<Entry>,
}

impl Program {
    /// Create one loaded native program.
    pub fn new(object: Arc<Object>, code: CodeMapping, entries: Vec<Entry>) -> Self {
        Self {
            object,
            code,
            entries,
        }
    }

    /// Borrow the native object.
    pub fn object(&self) -> &Object {
        self.object.as_ref()
    }

    /// Borrow the loaded code range.
    pub const fn code(&self) -> CodeMapping {
        self.code
    }

    /// Borrow the immutable program static memory.
    pub fn static_space(&self) -> &StaticSpace {
        &self.object.static_space
    }

    /// Borrow the program layout.
    pub fn layout(&self) -> &ProgramLayout {
        &self.object.layout
    }

    /// Return one entry by id.
    pub fn entry(&self, id: EntryId) -> Option<&Entry> {
        self.entries.get(id.0 as usize)
    }

    /// Return one native entry symbol by id.
    pub fn entry_symbol(&self, id: EntryId) -> Option<&EntrySymbol> {
        self.object.entry(id)
    }

    /// Return one entry by runtime name.
    pub fn entry_by_name(&self, name: &str) -> Option<&Entry> {
        let entry = self.object.entry_by_name(name)?;

        self.entry(entry.id)
    }

    /// Return all entries in entry id order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}
