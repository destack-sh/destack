use destack_core::{SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::CodeRange;

/// One native function linked into a Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// Typed native function body.
    pub body: Entry,
    /// Canonical engine-transition entry.
    pub entry: Entry,
}

/// One callable entry inside linked native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Entry {
    /// Linked code bytes.
    pub bytes: CodeRange,
}

impl Entry {
    /// Create one linked native entry.
    pub const fn new(bytes: CodeRange) -> Self {
        Self { bytes }
    }

    /// Validate this entry against linked code.
    pub(super) fn validate(self, bytes: usize) -> Result<(), SectionImageError> {
        self.bytes.validate(bytes)
    }
}

impl Function {
    /// Create one linked native function.
    pub const fn new(body: Entry, entry: Entry) -> Self {
        Self { body, entry }
    }

    /// Validate this function against linked code.
    pub(super) fn validate(self, bytes: usize) -> Result<(), SectionImageError> {
        self.body.validate(bytes)?;
        self.entry.validate(bytes)
    }
}
