use destack_core::{SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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

    /// Validate this trap against its code block.
    pub(super) fn validate(self, blocks: &[Block]) -> Result<(), SectionImageError> {
        let Some(block) = blocks.get(self.block.index()) else {
            return Err(SectionImageError::InvalidReference);
        };
        if self.offset >= block.byte_len() {
            return Err(SectionImageError::InvalidRange);
        }

        Ok(())
    }
}

impl CodeTrap {
    /// Create one linked native trap.
    pub const fn new(offset: u32, trap: abi::Trap) -> Self {
        Self { offset, trap }
    }

    /// Validate this trap against linked code.
    pub(super) fn validate(self, byte_len: usize) -> Result<(), SectionImageError> {
        if self.offset as usize >= byte_len {
            return Err(SectionImageError::InvalidRange);
        }

        Ok(())
    }
}
