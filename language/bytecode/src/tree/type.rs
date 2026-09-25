use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// A type identity, object-local before linking and Program-local afterward.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Parse one canonical object-local type name.
    pub fn from_name(name: &str) -> Option<Self> {
        let index = name.strip_prefix('t')?.parse::<u32>().ok()?;

        Some(Self(index))
    }

    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A layout identity, object-local before linking and Program-local afterward.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct LayoutId(pub u32);

impl LayoutId {
    /// Parse one canonical object-local layout name.
    pub fn from_name(name: &str) -> Option<Self> {
        let index = name.strip_prefix('l')?.parse::<u32>().ok()?;

        Some(Self(index))
    }

    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<TypeId>() == 4);
const _: () = assert!(size_of::<LayoutId>() == 4);
