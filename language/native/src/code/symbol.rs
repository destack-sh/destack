use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

use super::{Block, BlockId, Import};

/// Dense identity of one native object symbol.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct SymbolId(pub u32);

/// One addressable native object symbol.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum Symbol {
    /// One object-local code position.
    Block {
        /// Target block.
        block: BlockId,
        /// Byte offset inside the block.
        offset: u32,
    },
    /// Typed body of one object-local function.
    Function {
        /// Object-local function index.
        function: u32,
    },
    /// One platform function imported when native code is loaded.
    Import(Import),
    /// One object-local identity resolved during Program linking.
    Index(Index),
}

/// One object-local identity resolved into a dense Program index.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum Index {
    /// One runtime type.
    Type {
        /// Object-local MIR type identity.
        ty: u32,
    },
    /// One runtime layout.
    Layout {
        /// Object-local MIR type identity selecting the layout.
        ty: u32,
    },
    /// One Program function.
    Function {
        /// Object-local MIR function identity.
        function: u32,
    },
    /// One Program global.
    Global {
        /// Object-local MIR global identity.
        global: u32,
    },
    /// One dynamic dispatch table.
    Dynamic {
        /// Object-local dynamic table index.
        table: u32,
    },
    /// One heap allocation site.
    Allocation {
        /// Object-local allocation site index.
        site: u32,
    },
    /// One native frame map.
    Frame {
        /// Object-local native frame-map index.
        frame: u32,
    },
    /// One function-local profile counter.
    Counter {
        /// Object-local MIR function identity.
        function: u32,
        /// Function-local counter identity.
        counter: u32,
    },
    /// One function-local profile sampler.
    Sampler {
        /// Object-local MIR function identity.
        function: u32,
        /// Function-local sampler identity.
        sampler: u32,
    },
}

impl SymbolId {
    /// Return this id as a dense symbol index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl Symbol {
    /// Return whether this symbol fits its owning object tables.
    pub(super) fn fits(self, functions: usize, blocks: &[Block]) -> bool {
        match self {
            Self::Block { block, offset } => blocks
                .get(block.index())
                .is_some_and(|block| offset <= block.byte_len()),
            Self::Function { function } => (function as usize) < functions,
            Self::Import(_) | Self::Index(_) => true,
        }
    }
}
