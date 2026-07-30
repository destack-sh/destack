use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::ContentId;

use super::{Library, Object};

/// Durable native image payload.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum Image {
    /// Native functions are already linked into the current process.
    Resident,
    /// Native functions live in a loadable native library.
    Library(Library),
    /// Native functions live in a relocatable object artifact.
    Object(Object),
}

impl Image {
    /// Return all content ids referenced by this native image.
    pub fn content_ids(&self) -> Vec<ContentId> {
        match self {
            Self::Resident => Vec::new(),
            Self::Library(library) => library.content_ids(),
            Self::Object(object) => object.content_ids(),
        }
    }
}
