use serde::{Deserialize, Serialize};
use tspp_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use tspp_serde::Reflect;

use crate::{Symbol, TypeId};

/// Dense program global id.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum GlobalLocation {
    /// Immutable program constant storage.
    Constant = 0,
    /// Runtime-owned shared static storage.
    SharedStatic = 1,
    /// Worker-owned local static storage.
    LocalStatic = 2,
}

/// Program global entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct GlobalTable {
    /// Stable symbols keyed by dense global id.
    symbols: SectionSlice<Symbol>,
    /// Global entries keyed by dense global id.
    globals: SectionSlice<Global>,
}

impl GlobalTable {
    /// Pack one global table.
    pub(crate) fn pack(builder: GlobalTableBuilder, sections: &mut SectionBuilder) -> Self {
        Self {
            symbols: sections.insert(builder.symbols),
            globals: sections.insert(builder.globals),
        }
    }

    /// Return one stable global symbol.
    pub fn symbol(&self, sections: SectionImage<'_>, global: GlobalId) -> Option<Symbol> {
        sections.entries(self.symbols).get(global.index()).copied()
    }

    /// Return one global by id.
    pub fn get<'a>(&self, sections: SectionImage<'a>, global: GlobalId) -> Option<&'a Global> {
        sections.entries(self.globals).get(global.index())
    }

    /// Return all defined globals.
    pub fn iter<'a>(&'a self, sections: SectionImage<'a>) -> impl Iterator<Item = &'a Global> + 'a {
        sections.entries(self.globals).iter()
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
                (global.location == location).then_some((GlobalId(index as u32), global))
            })
    }
}

/// Mutable global table before section packing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalTableBuilder {
    /// Stable symbols in dense global id order.
    symbols: Vec<Symbol>,
    /// Globals in dense global id order.
    globals: Vec<Global>,
}

impl GlobalTableBuilder {
    /// Create one empty global table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set globals in dense global id order.
    pub fn globals(mut self, globals: impl IntoIterator<Item = (Symbol, Global)>) -> Self {
        (self.symbols, self.globals) = globals.into_iter().unzip();

        self
    }

    /// Return one global by its final dense id.
    pub fn get(&self, global: GlobalId) -> Option<&Global> {
        self.globals.get(global.index())
    }
}
