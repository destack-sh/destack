use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

use crate::abi;

use super::{Block, BlockId};

/// One object-local native trap site.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ObjectTrap {
    /// Object-local code block.
    pub block: BlockId,
    /// Trap site byte offset inside the block.
    pub offset: u32,
    /// Language trap reported by this instruction.
    pub trap: abi::Trap,
}

/// One native trap site inside linked code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct CodeTrap {
    /// Trap site byte offset inside linked code.
    pub offset: u32,
    /// Language trap reported by this instruction.
    pub trap: abi::Trap,
}

impl ObjectTrap {
    /// Create one object-local native trap.
    pub const fn new(block: BlockId, offset: u32, trap: abi::Trap) -> Self {
        Self {
            block,
            offset,
            trap,
        }
    }

    /// Return whether this trap belongs to one byte inside its code block.
    pub(super) fn fits(self, blocks: &[Block]) -> bool {
        blocks
            .get(self.block.index())
            .is_some_and(|block| self.offset < block.byte_len())
    }
}

impl CodeTrap {
    /// Create one linked native trap.
    pub const fn new(offset: u32, trap: abi::Trap) -> Self {
        Self { offset, trap }
    }

    /// Return whether this trap belongs to one byte inside linked code.
    pub(super) fn fits(self, byte_len: usize) -> bool {
        (self.offset as usize) < byte_len
    }
}
