use destack_core::{SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native function entry resolved by symbol name.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Entry {
    /// The native symbol exported by the linked image.
    pub symbol: StringId,
}

impl Entry {
    /// Create one native function entry.
    pub const fn new(symbol: StringId) -> Self {
        Self { symbol }
    }
}
