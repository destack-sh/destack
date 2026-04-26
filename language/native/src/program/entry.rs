use crate::{CodeOffset, EntryFn};

/// Dense native entry id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct EntryId(pub u32);

/// One named callable entry in an emitted native artifact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntryImage {
    /// The dense entry id.
    pub id: EntryId,
    /// The runtime entry name.
    pub name: String,
    /// The owning function.
    pub function: destack_engine::FunctionId,
    /// The code offset for this entry.
    pub offset: CodeOffset,
}

/// One named callable entry in a loaded native program.
#[derive(Clone)]
pub struct Entry {
    /// The dense entry id.
    pub id: EntryId,
    /// The loaded entry function pointer.
    pub function: EntryFn,
}

impl Entry {
    /// Load one emitted entry with its executable function.
    pub fn load(image: EntryImage, function: EntryFn) -> Self {
        Self {
            id: image.id,
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
