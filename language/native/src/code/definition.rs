use destack_core::{SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native function definition linked into a Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Definition {
    /// The internal native function body symbol.
    pub body: StringId,
    /// The native runtime entry symbol.
    pub entry: StringId,
    /// The compiled body byte length.
    pub body_byte_len: u32,
    /// Linked module owning this function.
    pub module: u32,
}

impl Definition {
    /// Create one linked native function definition.
    pub const fn new(body: StringId, entry: StringId, body_byte_len: u32, module: u32) -> Self {
        Self {
            body,
            entry,
            body_byte_len,
            module,
        }
    }
}
