use serde::{Deserialize, Serialize};

use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// Byte order for target scalar memory operations.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum Endian {
    /// Least significant byte first.
    Little,
    /// Most significant byte first.
    Big,
}

impl Endian {
    /// Return whether this byte order stores the least significant byte first.
    pub const fn is_little(self) -> bool {
        matches!(self, Self::Little)
    }
}

/// Pointer representation on the target.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct PointerLayout {
    /// Pointer size in bytes.
    pub size_bytes: u8,
    /// Pointer alignment in bytes.
    pub alignment_bytes: u8,
}

impl PointerLayout {
    /// Return pointer width in bits.
    #[inline]
    pub const fn width_bits(self) -> u16 {
        self.size_bytes as u16 * 8
    }
}

/// ABI layout for one target.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TargetLayout {
    /// Target byte order.
    pub endian: Endian,
    /// Target pointer layout.
    pub pointer: PointerLayout,
    /// Stack alignment in bytes.
    pub stack_alignment_bytes: u16,
}

impl Default for TargetLayout {
    fn default() -> Self {
        Self::for_pointer_bytes(8)
    }
}

impl TargetLayout {
    /// Create a default little-endian target layout for one pointer size.
    pub fn for_pointer_bytes(pointer_bytes: u8) -> Self {
        match pointer_bytes {
            4 | 8 => Self {
                endian: Endian::Little,
                pointer: PointerLayout {
                    size_bytes: pointer_bytes,
                    alignment_bytes: pointer_bytes,
                },
                stack_alignment_bytes: 16,
            },
            _ => {
                unreachable!("unsupported pointer size {pointer_bytes} bytes");
            }
        }
    }

    /// Return pointer size in bytes.
    #[inline]
    pub const fn pointer_bytes(self) -> u8 {
        self.pointer.size_bytes
    }

    /// Return pointer width in bits.
    #[inline]
    pub const fn pointer_bits(self) -> u16 {
        self.pointer.width_bits()
    }
}
