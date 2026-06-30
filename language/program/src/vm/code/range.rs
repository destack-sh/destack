use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::TypeId;

/// One lowered frame move slot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MoveSlot {
    /// The slot value type.
    pub ty: TypeId,
    /// Byte offset from the frame base.
    pub offset: u32,
    /// Slot byte length.
    pub byte_len: u32,
    /// Whether this slot stores one cell.
    pub is_cell: u32,
}

impl MoveSlot {
    /// Create one lowered frame move slot.
    pub const fn new(ty: TypeId, offset: u32, byte_len: u32, is_cell: bool) -> Self {
        Self {
            ty,
            offset,
            byte_len,
            is_cell: is_cell as u32,
        }
    }

    /// Return whether this slot stores one cell.
    pub const fn is_cell(self) -> bool {
        self.is_cell != 0
    }

    /// Return the slot byte length.
    pub const fn byte_len(self) -> u32 {
        self.byte_len
    }
}

/// Argument range within one function argument pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ArgumentRange {
    /// Start offset into the argument pool.
    pub start: u32,
    /// Number of arguments in the range.
    pub len: u32,
}

impl ArgumentRange {
    /// Create an empty argument range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice arguments from the pool for this range.
    #[inline(always)]
    pub fn slice<'a>(&self, pool: &'a [MoveSlot]) -> &'a [MoveSlot] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "argument pool out of bounds for range"
        );

        // return argument slice
        &pool[start..start + len]
    }
}

/// Move pair for parameter binding.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MovePair {
    /// Destination frame slot.
    pub dest: MoveSlot,
    /// Source frame slot or void fill.
    pub source: MoveSource,
}

/// Source for one lowered frame move.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MoveSource {
    /// Move source tag.
    pub tag: MoveSourceTag,
    /// Source slot when tag names a slot.
    pub slot: MoveSlot,
}

impl MoveSource {
    /// Create a source slot move.
    pub const fn slot(slot: MoveSlot) -> Self {
        Self {
            tag: MoveSourceTag::Slot,
            slot,
        }
    }

    /// Create a canonical void fill.
    pub const fn void() -> Self {
        Self {
            tag: MoveSourceTag::Void,
            slot: MoveSlot::new(TypeId(0), 0, 0, false),
        }
    }

    /// Return the source slot when this move copies from a slot.
    pub const fn as_slot(self) -> Option<MoveSlot> {
        if matches!(self.tag, MoveSourceTag::Slot) {
            Some(self.slot)
        } else {
            None
        }
    }

    /// Return whether this source writes the canonical void value.
    pub const fn is_void(self) -> bool {
        matches!(self.tag, MoveSourceTag::Void)
    }
}

/// Source kind for one lowered frame move.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MoveSourceTag {
    /// Move writes the canonical void value.
    #[default]
    Void = 0,
    /// Move copies from one frame slot.
    Slot = 1,
}

/// Move range within one function move pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MoveRange {
    /// Start offset into the move pool.
    pub start: u32,
    /// Number of pairs in the range.
    pub len: u32,
}

impl MoveRange {
    /// Create an empty move range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice pairs from the pool for this range.
    #[inline(always)]
    pub fn slice<'a>(&self, pool: &'a [MovePair]) -> &'a [MovePair] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "move pool out of bounds for range"
        );

        // return move slice
        &pool[start..start + len]
    }
}

// SAFETY: VM move entries are repr(C), Copy, and contain only fixed program entries.
unsafe impl SectionEntry for MoveSlot {}
unsafe impl SectionEntry for ArgumentRange {}
unsafe impl SectionEntry for MovePair {}
unsafe impl SectionEntry for MoveSource {}
unsafe impl SectionEntry for MoveSourceTag {}
unsafe impl SectionEntry for MoveRange {}
