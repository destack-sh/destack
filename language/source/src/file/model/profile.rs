use serde::{Deserialize, Serialize};

/// Unique identifier for profiles.
///
/// A profile represents a semantic configuration (comptime world) that determines
/// which symbols exist and how types resolve. Multiple targets can share the same
/// profile, allowing them to share canonical DIR.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(pub u128);

impl std::fmt::Debug for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p{:032x}", self.0)
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p{:032x}", self.0)
    }
}

impl ProfileId {
    /// Wrap an id as a ProfileId.
    pub fn new(id: u128) -> Self {
        Self(id)
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u128 {
        self.0
    }
}
