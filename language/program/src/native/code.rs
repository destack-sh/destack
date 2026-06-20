use serde::{Deserialize, Serialize};

use destack_source::ContentId;

use super::{Abi, CodeMap, Entry, EntryTable, Image, ImportTable, Resume};

/// Durable native code produced for one program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Code {
    /// The native ABI required by this code.
    pub abi: Abi,
    /// The native image kind.
    pub image: Image,
    /// Native imports required by this code.
    pub import: ImportTable,
    /// Native code map for safepoints and deoptimization.
    pub map: CodeMap,
    /// Native entries keyed by program ids.
    pub entry: EntryTable,
}

impl Code {
    /// Create one native code payload.
    pub fn new(
        abi: Abi,
        image: Image,
        import: ImportTable,
        map: CodeMap,
        entry: EntryTable,
    ) -> Self {
        Self {
            abi,
            image,
            import,
            map,
            entry,
        }
    }

    /// Return all content ids referenced by this native code.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.image.content_ids()
    }

    /// Return native function entries in dense program function id order.
    pub fn function_entry(&self) -> &[Option<Entry>] {
        &self.entry.function
    }

    /// Return native resume entries in dense frame state id order.
    pub fn resume_entry(&self) -> &[Option<Resume>] {
        &self.entry.resume
    }
}
