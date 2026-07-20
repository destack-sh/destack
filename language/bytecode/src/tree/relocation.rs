use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Symbol;

/// One symbolic operand in an instruction stream.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct InstructionRelocation {
    /// The first byte of the encoded operand.
    pub byte_offset: u32,
    /// The referenced object-local symbol.
    pub symbol: Symbol,
}

impl InstructionRelocation {
    /// Create one symbolic operand relocation.
    pub const fn new(byte_offset: u32, symbol: Symbol) -> Self {
        Self {
            byte_offset,
            symbol,
        }
    }

    /// Rebase this relocation into its containing byte section.
    pub const fn rebase(self, byte_offset: u32) -> Self {
        Self::new(self.byte_offset + byte_offset, self.symbol)
    }
}

/// One symbolic operand in the constant byte section.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ConstantRelocation {
    /// The first byte of the encoded operand.
    pub byte_offset: u32,
    /// The referenced object-local symbol.
    pub symbol: Symbol,
    /// The signed byte addend applied to the linked symbol.
    pub addend: i64,
}

impl ConstantRelocation {
    /// Create one immutable constant relocation.
    pub const fn new(byte_offset: u32, symbol: Symbol, addend: i64) -> Self {
        Self {
            byte_offset,
            symbol,
            addend,
        }
    }

    /// Rebase this relocation into its containing byte section.
    pub const fn rebase(self, byte_offset: u32) -> Self {
        Self::new(self.byte_offset + byte_offset, self.symbol, self.addend)
    }
}

const _: () = assert!(size_of::<InstructionRelocation>() == 12);
const _: () = assert!(size_of::<ConstantRelocation>() == 24);
