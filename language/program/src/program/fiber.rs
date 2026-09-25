use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Runtime identity for one fiber.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FiberId(u64);

impl FiberId {
    /// Create one fiber identity from its slot and generation.
    pub const fn new(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << u32::BITS) | index as u64)
    }

    /// Return the fiber table slot.
    pub const fn index(self) -> u32 {
        self.0 as u32
    }

    /// Return the fiber slot generation.
    pub const fn generation(self) -> u32 {
        (self.0 >> u32::BITS) as u32
    }

    /// Return the scalar handle passed through program code.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Decode one scalar handle passed through program code.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}
