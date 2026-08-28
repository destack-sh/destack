use destack_core::{SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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

    /// Validate this relocation against the encoded bytecode.
    pub(crate) fn validate(self, code_byte_len: usize) -> Result<(), SectionImageError> {
        if self.reserved != [0; 3] {
            return Err(SectionImageError::InvalidEntry);
        }
        if !matches!(
            self.tag,
            RelocationTag::TYPE
                | RelocationTag::LAYOUT
                | RelocationTag::FUNCTION
                | RelocationTag::GLOBAL
                | RelocationTag::DYNAMIC
                | RelocationTag::ALLOCATION
                | RelocationTag::COUNTER
                | RelocationTag::SAMPLER
        ) {
            return Err(SectionImageError::InvalidEntry);
        }

        let byte_len = size_of::<u32>() as u32;
        let Some(end) = self.byte_offset.checked_add(byte_len) else {
            return Err(SectionImageError::InvalidRange);
        };
        if end as usize > code_byte_len {
            return Err(SectionImageError::InvalidRange);
        }
        Ok(())
    }
}

const _: () = assert!(size_of::<Relocation>() == 8);
const _: () = assert!(size_of::<RelocationTag>() == 1);
