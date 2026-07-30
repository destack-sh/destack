use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Stable declaration identity across Program versions.
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
pub struct Symbol(u64);

impl Symbol {
    /// Restore one symbol from its persistent bits.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the persistent symbol bits.
    pub const fn raw(self) -> u64 {
        self.0
    }
}
