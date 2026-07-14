use destack_core::{SectionEntry, SectionImage, SectionPacker, SectionSlice};
use destack_heap::DropId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FunctionId;

/// One executable destructor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DropEntry {
    /// The destructor function.
    pub function: FunctionId,
}

// SAFETY: drop entries contain fixed-width section values.
unsafe impl SectionEntry for DropEntry {}

/// Destructors carried by one program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DropTable {
    /// Complete drop entries indexed by DropId.
    entries: SectionSlice<DropEntry>,
}

impl DropTable {
    /// Pack destructors into one program table.
    pub fn pack(sections: &mut SectionPacker, entries: Vec<DropEntry>) -> Self {
        Self {
            entries: sections.insert(entries),
        }
    }

    /// Return one complete drop entry.
    pub fn entry(&self, sections: SectionImage<'_>, drop: DropId) -> Option<DropEntry> {
        sections.entries(self.entries).get(drop.index()).copied()
    }
}
