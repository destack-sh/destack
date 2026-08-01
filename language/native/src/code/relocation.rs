use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::SymbolId;

/// One unresolved relocation inside an object code block.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Relocation {
    /// Byte offset inside the containing block.
    pub offset: u32,
    /// Object-local target symbol.
    pub target: SymbolId,
    /// Value added to the resolved target address.
    pub addend: i64,
    /// Machine relocation encoding.
    pub kind: RelocationKind,
}

impl Relocation {
    /// Create one object-local relocation.
    pub const fn new(offset: u32, target: SymbolId, addend: i64, kind: RelocationKind) -> Self {
        Self {
            offset,
            target,
            addend,
            kind,
        }
    }

    /// Return whether this relocation fits its block and symbol tables.
    pub(super) fn fits(self, byte_len: usize, symbols: usize) -> bool {
        let target_fits = self.target.index() < symbols;
        let bytes_fit = self
            .offset
            .checked_add(self.kind.byte_len())
            .is_some_and(|end| end as usize <= byte_len);

        target_fits && bytes_fit
    }
}

impl RelocationKind {
    /// Return the number of patched bytes.
    pub const fn byte_len(self) -> u32 {
        match self {
            Self::Relative32
            | Self::X86PcRelative32
            | Self::X86CallRelative32
            | Self::Aarch64Call26
            | Self::Aarch64Page21
            | Self::Aarch64Low12 => 4,
            Self::RiscvCall => 8,
        }
    }
}

/// Machine relocation encoding inside a relocatable native object.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum RelocationKind {
    /// Write one 32-bit PC-relative address.
    Relative32 = 0x00,

    /// Write one x86-64 32-bit PC-relative address.
    X86PcRelative32 = 0x10,
    /// Write one x86-64 32-bit PC-relative call target.
    X86CallRelative32 = 0x11,

    /// Write one AArch64 26-bit branch target.
    Aarch64Call26 = 0x20,
    /// Write one AArch64 page-relative address.
    Aarch64Page21 = 0x21,
    /// Write one AArch64 low 12-bit address.
    Aarch64Low12 = 0x22,

    /// Write one RISC-V call pair.
    RiscvCall = 0x30,
}
