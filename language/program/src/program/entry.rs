use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::FunctionId;

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
