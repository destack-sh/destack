use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One object-local bytecode symbol.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Symbol {
    /// The symbol category.
    pub tag: SymbolTag,
    /// Reserved symbol bytes.
    reserved: [u8; 3],
    /// The object-local index in the corresponding table.
    pub index: u32,
}

impl Symbol {
    /// Create one runtime type symbol.
    pub const fn ty(index: u32) -> Self {
        Self::new(SymbolTag::TYPE, index)
    }

    /// Create one global symbol.
    pub const fn global(index: u32) -> Self {
        Self::new(SymbolTag::GLOBAL, index)
    }

    /// Create one function symbol.
    pub const fn function(index: u32) -> Self {
        Self::new(SymbolTag::FUNCTION, index)
    }

    /// Create one function type symbol.
    pub const fn function_type(index: u32) -> Self {
        Self::new(SymbolTag::FUNCTION_TYPE, index)
    }

    /// Create one immutable constant symbol.
    pub const fn constant(index: u32) -> Self {
        Self::new(SymbolTag::CONSTANT, index)
    }

    /// Create one symbol.
    const fn new(tag: SymbolTag, index: u32) -> Self {
        Self {
            tag,
            reserved: [0; 3],
            index,
        }
    }
}

/// The category of one bytecode symbol.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SymbolTag(pub u8);

impl SymbolTag {
    /// A runtime type.
    pub const TYPE: Self = Self(0);
    /// A global.
    pub const GLOBAL: Self = Self(1);
    /// A function.
    pub const FUNCTION: Self = Self(2);
    /// A function type.
    pub const FUNCTION_TYPE: Self = Self(3);
    /// An immutable constant.
    pub const CONSTANT: Self = Self(4);

    /// Return whether this symbol category is defined by the bytecode format.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::CONSTANT.0
    }
}

const _: () = assert!(size_of::<Symbol>() == 8);
const _: () = assert!(size_of::<SymbolTag>() == 1);
