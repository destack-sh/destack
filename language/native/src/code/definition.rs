use serde::{Deserialize, Serialize};
use tspp_core::{EntryRange, EntryStore, SectionEntry};
use tspp_serde::Reflect;

use super::BlockId;

/// One native function definition inside a relocatable object.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Definition {
    /// Typed native function body.
    pub body: BlockId,
    /// Canonical engine-transition entry.
    pub entry: BlockId,
    /// Coroutine resume entries.
    resumes: EntryRange<Resume>,
}

/// One object-local coroutine resume entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Resume {
    /// Object-local canonical frame state.
    pub state: u32,
    /// Native code reconstructing and resuming that state.
    pub block: BlockId,
}

/// One native function definition under construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionBuilder {
    /// Typed native function body.
    body: BlockId,
    /// Canonical engine-transition entry.
    entry: BlockId,
    /// Coroutine resume entries.
    resumes: Vec<Resume>,
}

impl Definition {
    /// Return coroutine resume entries.
    pub fn resumes(self, resumes: &[Resume]) -> &[Resume] {
        self.resumes.slice(resumes)
    }

    /// Return whether every referenced block and resume exists.
    pub(super) fn ranges_fit(self, blocks: usize, resumes: &[Resume]) -> bool {
        self.body.index() < blocks
            && self.entry.index() < blocks
            && self.resumes.fits(resumes.len())
            && self
                .resumes(resumes)
                .iter()
                .all(|resume| resume.block.index() < blocks)
    }
}

impl Resume {
    /// Create one object-local coroutine resume entry.
    pub const fn new(state: u32, block: BlockId) -> Self {
        Self { state, block }
    }
}

impl DefinitionBuilder {
    /// Create one native function definition.
    pub fn new(body: BlockId, entry: BlockId) -> Self {
        Self {
            body,
            entry,
            resumes: Vec::new(),
        }
    }

    /// Return the typed native function body block.
    pub const fn body(&self) -> BlockId {
        self.body
    }

    /// Return the canonical engine-transition entry block.
    pub const fn entry(&self) -> BlockId {
        self.entry
    }

    /// Set coroutine resume entries.
    pub fn resumes(mut self, resumes: impl IntoIterator<Item = Resume>) -> Self {
        self.resumes = resumes.into_iter().collect();

        self
    }

    /// Pack this definition into flattened object columns.
    pub(super) fn build(self, resumes: &mut EntryStore<Resume>) -> Definition {
        Definition {
            body: self.body,
            entry: self.entry,
            resumes: resumes.append(self.resumes),
        }
    }
}
