use serde::{Deserialize, Serialize};
use tspp_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use tspp_serde::Reflect;

use crate::FunctionId;

/// Module initializers in dependency order, the entry module's last.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct InitializerTable {
    /// The initializer functions to run before the program is live.
    initializers: SectionSlice<EntryPoint>,
}

impl InitializerTable {
    /// Pack the ordered initializers into one program table.
    pub(crate) fn pack(sections: &mut SectionBuilder, initializers: Vec<EntryPoint>) -> Self {
        Self {
            initializers: sections.insert(initializers),
        }
    }

    /// Return the ordered initializers.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [EntryPoint] {
        sections.entries(self.initializers)
    }
}

/// One program entrypoint id.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct EntryPoint(FunctionId);

impl EntryPoint {
    /// Create one program entrypoint.
    pub const fn new(index: u32) -> Self {
        Self(FunctionId(index))
    }

    /// Return the entrypoint function.
    pub const fn function(self) -> FunctionId {
        self.0
    }

    /// Return the entrypoint index.
    pub const fn index(self) -> u32 {
        self.0.0
    }
}

impl From<FunctionId> for EntryPoint {
    /// Convert a program function id into an entrypoint.
    fn from(function: FunctionId) -> Self {
        Self(function)
    }
}
