use serde::{Deserialize, Serialize};
use tspp_core::{SectionEntry, StringId};
use tspp_serde::Reflect;

/// One WebAssembly export implementing a Program function.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Entry {
    /// The generated WebAssembly export name.
    pub export: StringId,
}

impl Entry {
    /// Create one exported WebAssembly function entry.
    pub const fn new(export: StringId) -> Self {
        Self { export }
    }
}
