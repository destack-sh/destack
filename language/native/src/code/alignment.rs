use serde::{Deserialize, Serialize};
use tspp_core::SectionEntry;
use tspp_serde::Reflect;

/// One native code alignment in bytes.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct Alignment(u32);

impl Default for Alignment {
    /// Return one-byte alignment.
    fn default() -> Self {
        Self::ONE
    }
}

impl Alignment {
    /// One-byte alignment.
    pub const ONE: Self = Self(1);
    /// Two-byte alignment.
    pub const TWO: Self = Self(2);

    /// Create one power-of-two native code alignment.
    pub const fn new(bytes: u32) -> Option<Self> {
        let alignment = Self(bytes);

        if alignment.is_valid() {
            Some(alignment)
        } else {
            None
        }
    }

    /// Return the alignment in bytes.
    pub const fn bytes(self) -> u32 {
        self.0
    }

    /// Return whether this alignment is valid.
    pub(super) const fn is_valid(self) -> bool {
        self.0.is_power_of_two()
    }
}
