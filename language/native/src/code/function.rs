use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::CodeRange;

/// One native function linked into a Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// Typed internal function body.
    pub body: Entry,
    /// Uniform runtime entry wrapper.
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

    /// Return whether this entry fits its linked columns.
    pub(super) fn ranges_fit(self, bytes: usize) -> bool {
        self.bytes.fits(bytes)
    }
}

impl Function {
    /// Create one linked native function.
    pub const fn new(body: Entry, entry: Entry) -> Self {
        Self { body, entry }
    }

    /// Return whether this function fits its linked columns.
    pub(super) fn ranges_fit(self, bytes: usize) -> bool {
        self.body.ranges_fit(bytes) && self.entry.ranges_fit(bytes)
    }
}
