use destack_core::{SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One nominal type symbol referenced by a bytecode object.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Type {
    /// The stable type symbol name.
    pub name: StringId,
}

/// An object-local type symbol id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Return this id as a dense object index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<Type>() == 8);
const _: () = assert!(size_of::<TypeId>() == 4);
