use destack_core::{EntryRange, Optional, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One immutable byte sequence embedded in a bytecode object.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Constant {
    /// The stable declaration name when explicitly named.
    pub name: Optional<StringId>,
    /// The immutable constant value.
    pub value: ConstantValue,
}

/// One immutable constant value.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ConstantValue {
    /// The required byte alignment.
    pub alignment_bytes: u32,
    /// The immutable bytes in the object constant byte section.
    pub bytes: EntryRange<u8>,
}

impl Constant {
    /// Borrow this constant's bytes.
    pub fn bytes<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        self.value.bytes(bytes)
    }
}

impl ConstantValue {
    /// Borrow this value from its containing constant byte section.
    pub fn bytes<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        self.bytes.slice(bytes)
    }
}

/// An object-local constant id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ConstantId(pub u32);

impl ConstantId {
    /// Return this id as a dense object index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<Constant>() == 32);
const _: () = assert!(size_of::<ConstantValue>() == 12);
const _: () = assert!(size_of::<ConstantId>() == 4);
