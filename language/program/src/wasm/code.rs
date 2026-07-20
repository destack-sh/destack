use destack_core::SectionEntry;
use destack_serde::Reflect;
use destack_source::ContentId;
use serde::{Deserialize, Serialize};

/// Linked WebAssembly code produced for one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// The linked WebAssembly module.
    pub module: ContentId,
}

impl Code {
    /// Create linked WebAssembly code.
    pub fn new(module: ContentId) -> Self {
        Self { module }
    }

    /// Return all content ids referenced by this WebAssembly code.
    pub fn content_ids(&self) -> Vec<ContentId> {
        vec![self.module]
    }
}
