use destack_core::{SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One byte range inside native code.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeRange {
    /// Byte offset from the start of the containing code.
    pub offset: u32,
    /// Number of bytes in the range.
    pub byte_len: u32,
}

impl CodeRange {
    /// Create one native code range.
    pub const fn new(offset: u32, byte_len: u32) -> Self {
        Self { offset, byte_len }
    }

    /// Return the exclusive byte end.
    pub const fn end(self) -> u32 {
        self.offset + self.byte_len
    }

    /// Return the exclusive byte end when representable.
    pub(super) const fn checked_end(self) -> Option<u32> {
        self.offset.checked_add(self.byte_len)
    }

    /// Return whether this range ends before another range begins.
    pub(super) const fn precedes(self, other: Self) -> bool {
        match self.checked_end() {
            Some(end) => end <= other.offset,
            None => false,
        }
    }

    /// Validate this range against one byte region.
    pub fn validate(self, byte_len: usize) -> Result<(), SectionImageError> {
        let Some(end) = self.checked_end() else {
            return Err(SectionImageError::InvalidRange);
        };
        if end as usize > byte_len {
            return Err(SectionImageError::InvalidRange);
        }

        Ok(())
    }

    /// Borrow this range from its containing code.
    pub fn bytes(self, code: &[u8]) -> &[u8] {
        &code[self.offset as usize..self.end() as usize]
    }
}
