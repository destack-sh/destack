use destack_core::SectionEntry;
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

/// Relocatable WebAssembly object.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Object {
    /// The encoded WebAssembly object bytes.
    pub content: ContentId,
}

impl Object {
    /// Create one relocatable WebAssembly object.
    pub const fn new(content: ContentId) -> Self {
        Self { content }
    }
}
