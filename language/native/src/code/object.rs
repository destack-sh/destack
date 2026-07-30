use destack_core::{Optional, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::ContentId;

/// Relocatable native object image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Object {
    /// The object file format.
    pub format: ObjectFormat,
    /// The object file bytes.
    pub content: ContentId,
    /// Native unwind tables bytes.
    pub unwind: Optional<ContentId>,
}

impl Object {
    /// Create one relocatable native object image.
    pub fn new(format: ObjectFormat, content: ContentId, unwind: Option<ContentId>) -> Self {
        Self {
            format,
            content,
            unwind: unwind.into(),
        }
    }

    /// Return all content ids referenced by this object image.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut ids = Vec::with_capacity(2);
        ids.push(self.content);

        if let Some(unwind) = self.unwind.get() {
            ids.push(unwind);
        }

        ids
    }
}

/// Native object file format.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum ObjectFormat {
    /// Executable and Linkable Format object.
    Elf,
    /// Mach object file.
    MachO,
    /// Common Object File Format object.
    Coff,
}
