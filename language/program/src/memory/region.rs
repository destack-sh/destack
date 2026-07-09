use destack_core::{Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::TypeId;

/// Dense program global id.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GlobalId(pub u32);

impl GlobalId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for GlobalId {
    /// Convert one raw program global id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<GlobalId> for u32 {
    /// Convert one program global id into its raw value.
    fn from(id: GlobalId) -> Self {
        id.0
    }
}

/// Storage location for one program global.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum GlobalLocation {
    /// Immutable program constant storage.
    Constant = 0,
    /// Runtime-owned shared static storage.
    SharedStatic = 1,
    /// Worker-owned local static storage.
    LocalStatic = 2,
}

/// Program global record.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Global {
    /// Static storage location for this global.
    pub location: GlobalLocation,
    /// The byte offset inside the static image or space.
    pub offset: u64,
    /// The global byte length.
    pub byte_len: u64,
    /// The global value type.
    pub ty: TypeId,
    /// Whether this global allows stores.
    pub is_mutable: u32,
}

impl Global {
    /// Create one defined global.
    pub const fn new(
        location: GlobalLocation,
        offset: usize,
        byte_len: usize,
        ty: TypeId,
        is_mutable: bool,
    ) -> Self {
        Self {
            location,
            offset: offset as u64,
            byte_len: byte_len as u64,
            ty,
            is_mutable: is_mutable as u32,
        }
    }

    /// Return the byte offset inside the static image or space.
    pub const fn offset(self) -> usize {
        self.offset as usize
    }

    /// Return the global byte length.
    pub const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return whether this global allows stores.
    pub const fn is_mutable(self) -> bool {
        self.is_mutable != 0
    }
}

/// Global table carried by one program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalTable {
    /// Global records keyed by dense global id.
    globals: SectionSlice<Optional<Global>>,
}

impl GlobalTable {
    /// Pack one global table.
    pub fn pack(sections: &mut SectionPacker, globals: Vec<Option<Global>>) -> Self {
        let globals = globals.into_iter().map(Optional::from).collect::<Vec<_>>();
        let globals = sections.insert(globals);

        Self { globals }
    }

    /// Return one global by id.
    pub fn get<'a>(&self, sections: SectionImage<'a>, global: GlobalId) -> Option<&'a Global> {
        sections
            .entries(self.globals)
            .get(global.index())
            .and_then(Optional::as_ref)
    }

    /// Return all defined globals.
    pub fn iter<'a>(&'a self, sections: SectionImage<'a>) -> impl Iterator<Item = &'a Global> + 'a {
        sections
            .entries(self.globals)
            .iter()
            .filter_map(Optional::as_ref)
    }

    /// Return defined globals in one storage location.
    pub fn iter_location<'a>(
        &'a self,
        sections: SectionImage<'a>,
        location: GlobalLocation,
    ) -> impl Iterator<Item = (GlobalId, &'a Global)> + 'a {
        sections
            .entries(self.globals)
            .iter()
            .enumerate()
            .filter_map(move |(index, global)| {
                let global = global.as_ref()?;

                (global.location == location).then_some((GlobalId(index as u32), global))
            })
    }
}

// SAFETY: global ids and records are fixed-width program entries.
unsafe impl SectionEntry for GlobalId {}
unsafe impl SectionEntry for GlobalLocation {}
unsafe impl SectionEntry for Global {}
