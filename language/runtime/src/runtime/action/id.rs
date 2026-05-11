use destack_core::fnv1a_128;
use serde::{Deserialize, Serialize};

/// Stable identifier for one canonical host action name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HostActionId(
    /// Stable hash of the canonical action string.
    pub u128,
);

impl HostActionId {
    /// Build one action identifier from one action name.
    pub const fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}
