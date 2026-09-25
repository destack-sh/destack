use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// The Program identity selected by one relocation.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct RelocationTag(pub u8);

impl RelocationTag {
    /// A runtime type.
    pub const TYPE: Self = Self(0);
    /// A runtime layout.
    pub const LAYOUT: Self = Self(1);
    /// A function.
    pub const FUNCTION: Self = Self(2);
    /// A global.
    pub const GLOBAL: Self = Self(3);
    /// A dynamic dispatch table.
    pub const DYNAMIC: Self = Self(5);
    /// An allocation site.
    pub const ALLOCATION: Self = Self(6);
    /// A profile counter.
    pub const COUNTER: Self = Self(8);
    /// A profile sampler.
    pub const SAMPLER: Self = Self(9);
}

/// One relocatable identity operand in an instruction stream.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Relocation {
    /// The first byte of the encoded identity operand.
    pub byte_offset: u32,
    /// The Program identity selected by the operand.
    pub tag: RelocationTag,
    /// Reserved relocation bytes.
    reserved: [u8; 3],
}

impl Relocation {
    /// Create one instruction operand relocation.
    pub const fn new(byte_offset: u32, tag: RelocationTag) -> Self {
        Self {
            byte_offset,
            tag,
            reserved: [0; 3],
        }
    }

    /// Rebase this relocation into its containing code section.
    pub const fn rebase(self, byte_offset: u32) -> Self {
        Self::new(self.byte_offset + byte_offset, self.tag)
    }
}

const _: () = assert!(size_of::<Relocation>() == 8);
const _: () = assert!(size_of::<RelocationTag>() == 1);
