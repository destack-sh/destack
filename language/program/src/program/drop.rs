use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_heap::DropId;
use destack_mir::{Space, Storage};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FunctionId;

/// Placement-specific destructors for one program type.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DropEntry {
    /// The destructor for values retained in activation frames.
    pub frame: Optional<FunctionId>,
    /// The destructor for worker-local heap allocations.
    pub local: Optional<FunctionId>,
    /// The destructor for runtime-shared heap allocations.
    pub shared: Optional<FunctionId>,
}

impl DropEntry {
    /// Return the destructor for one storage placement.
    pub fn destructor(self, storage: Storage) -> Option<FunctionId> {
        match storage {
            Storage::Frame => self.frame.get(),
            Storage::Heap(Space::Local) => self.local.get(),
            Storage::Heap(Space::Shared) => self.shared.get(),
            Storage::Static(_) => None,
            Storage::Heap(Space::Constant | Space::Parameter(_) | Space::Join(_) | Space::Of(_))
            | Storage::Parameter(_)
            | Storage::Bound { .. }
            | Storage::Join(_) => {
                unreachable!("a program drop in an open space")
            }
        }
    }
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
