use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_heap::DropId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FunctionId;

/// One program destructor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DropEntry {
    /// The destructor function.
    pub function: FunctionId,
}

/// Destructors carried by one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DropTable {
    /// Complete drop entries indexed by DropId.
    entries: SectionSlice<DropEntry>,
}

impl DropTable {
    /// Pack destructors into one program table.
    pub(crate) fn pack(sections: &mut SectionBuilder, entries: Vec<DropEntry>) -> Self {
        Self {
            entries: sections.insert(entries),
        }
    }

    /// Return one complete drop entry.
    pub fn entry(&self, sections: SectionImage<'_>, drop: DropId) -> Option<DropEntry> {
        sections.entries(self.entries).get(drop.index()).copied()
    }
}
