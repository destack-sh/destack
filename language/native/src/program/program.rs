use std::sync::Arc;

use crate::{CodeMapping, Entry, EntryId, EntryImage, Object};

/// Native executable artifact loaded by a worker.
#[derive(Debug, Clone)]
pub struct Program {
    /// The emitted object backing this loaded program.
    object: Arc<Object>,
    /// The loaded executable code range.
    code: CodeMapping,
    /// The named entry table.
    entries: Vec<Entry>,
}

impl Program {
    /// Create one loaded native program artifact.
    pub fn new(object: Arc<Object>, code: CodeMapping, entries: Vec<Entry>) -> Self {
        Self {
            object,
            code,
            entries,
        }
    }

    /// Borrow the emitted object backing this loaded program.
    pub fn object(&self) -> &Object {
        self.object.as_ref()
    }

    /// Borrow the loaded executable code range.
    pub const fn code(&self) -> CodeMapping {
        self.code
    }

    /// Borrow the immutable program static memory.
    pub fn static_space(&self) -> &destack_engine::StaticSpace {
        &self.object.static_space
    }

    /// Borrow the program metadata.
    pub fn metadata(&self) -> &destack_engine::ProgramMetadata {
        &self.object.metadata
    }

    /// Return one entry by id.
    pub fn entry(&self, id: EntryId) -> Option<&Entry> {
        self.entries.get(id.0 as usize)
    }

    /// Return one emitted entry by id.
    pub fn entry_image(&self, id: EntryId) -> Option<&EntryImage> {
        self.object.entry(id)
    }

    /// Return one entry by runtime name.
    pub fn entry_by_name(&self, name: &str) -> Option<&Entry> {
        let image = self.object.entry_by_name(name)?;

        self.entry(image.id)
    }

    /// Return all entries in entry id order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}
