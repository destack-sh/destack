use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// One byte range inside a linked native code image.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CodeRange {
    /// Byte offset from the start of the code image.
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

    /// Return whether this range fits one code image.
    pub fn fits(self, byte_len: usize) -> bool {
        self.offset
            .checked_add(self.byte_len)
            .is_some_and(|end| end as usize <= byte_len)
    }

    /// Borrow this range from one code image.
    pub fn bytes(self, code: &[u8]) -> &[u8] {
        &code[self.offset as usize..self.end() as usize]
    }
}
