use destack_core::{Optional, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ConstantId, Linkage, TypeId};

/// One bytecode global declaration or definition.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Global {
    /// The stable global symbol name.
    pub name: StringId,
    /// The global linkage.
    pub linkage: Linkage,
    /// The global static storage location.
    pub location: GlobalLocation,
    /// Whether instructions may store into this global.
    is_mutable: u8,
    /// Reserved global byte.
    reserved: u8,
    /// The stored value type.
    pub ty: TypeId,
    /// The initializer bytes, or zero initialization when absent.
    pub initializer: Optional<ConstantId>,
}

impl Global {
    /// Create one bytecode global declaration or definition.
    pub const fn new(
        name: StringId,
        linkage: Linkage,
        location: GlobalLocation,
        is_mutable: bool,
        ty: TypeId,
        initializer: Optional<ConstantId>,
    ) -> Self {
        Self {
            name,
            linkage,
            location,
            is_mutable: is_mutable as u8,
            reserved: 0,
            ty,
            initializer,
        }
    }

    /// Return whether instructions may store into this global.
    pub const fn is_mutable(&self) -> bool {
        self.is_mutable != 0
    }

    /// Return the initializer bytes when this definition has a constant initializer.
    pub fn initializer(&self) -> Option<ConstantId> {
        self.initializer.get()
    }
}

/// An object-local global id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct GlobalId(pub u32);

impl GlobalId {
    /// Return this id as a dense object index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// The static storage selected by one bytecode global.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct GlobalLocation(pub u8);

impl GlobalLocation {
    /// Immutable program storage.
    pub const CONSTANT: Self = Self(0);
    /// Runtime-owned static storage.
    pub const SHARED_STATIC: Self = Self(1);
    /// Worker-owned static storage.
    pub const LOCAL_STATIC: Self = Self(2);

    /// Return whether this location is defined by the bytecode object format.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::LOCAL_STATIC.0
    }
}

const _: () = assert!(size_of::<Global>() == 24);
const _: () = assert!(size_of::<GlobalId>() == 4);
const _: () = assert!(size_of::<GlobalLocation>() == 1);
