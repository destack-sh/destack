use serde::{Deserialize, Serialize};
use tspp_core::{EntryRange, EntryStore, SectionEntry};
use tspp_serde::Reflect;

use super::{Alignment, Relocation};

/// Dense identity of one native code block inside an object.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct BlockId(pub u32);

/// One independently placed native image block.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Block {
    /// Block bytes inside the object byte column.
    bytes: EntryRange<u8>,
    /// Block-relative relocations.
    relocations: EntryRange<Relocation>,
    /// Required executable alignment.
    pub alignment: Alignment,
}

/// One native code block under construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBuilder {
    /// Native machine-code bytes.
    bytes: Vec<u8>,
    /// Required executable alignment.
    alignment: Alignment,
    /// Block-relative relocations.
    relocations: Vec<Relocation>,
}

impl BlockId {
    /// Return this id as a dense block index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl Block {
    /// Return the block byte length.
    pub const fn byte_len(self) -> u32 {
        self.bytes.len
    }

    /// Return this block's bytes.
    pub fn bytes(self, bytes: &[u8]) -> &[u8] {
        self.bytes.slice(bytes)
    }

    /// Return block-relative relocations.
    pub fn relocations(self, relocations: &[Relocation]) -> &[Relocation] {
        self.relocations.slice(relocations)
    }

    /// Return whether every block range fits its owning column.
    pub(super) fn ranges_fit(
        self,
        symbols: usize,
        bytes: usize,
        relocations: &[Relocation],
    ) -> bool {
        if !self.alignment.is_valid()
            || !self.bytes.fits(bytes)
            || !self.relocations.fits(relocations.len())
        {
            return false;
        }

        // check every relocation relative to this block
        let byte_len = self.bytes.len as usize;

        self.relocations(relocations)
            .iter()
            .all(|relocation| relocation.fits(byte_len, symbols))
    }
}

impl BlockBuilder {
    /// Create one native image block.
    pub fn new(bytes: impl Into<Vec<u8>>, alignment: Alignment) -> Self {
        Self {
            bytes: bytes.into(),
            alignment,
            relocations: Vec::new(),
        }
    }

    /// Set block-relative relocations.
    pub fn relocations(mut self, relocations: impl IntoIterator<Item = Relocation>) -> Self {
        self.relocations = relocations.into_iter().collect();

        self
    }

    /// Pack this block into flattened object columns.
    pub(super) fn build(
        self,
        bytes: &mut Vec<u8>,
        relocations: &mut EntryStore<Relocation>,
    ) -> Block {
        let offset = bytes
            .len()
            .next_multiple_of(self.alignment.bytes() as usize);
        bytes.resize(offset, 0);
        let byte_len = self.bytes.len();
        bytes.extend_from_slice(&self.bytes);

        Block {
            bytes: EntryRange::new(offset as u32, byte_len as u32),
            relocations: relocations.append(self.relocations),
            alignment: self.alignment,
        }
    }
}
