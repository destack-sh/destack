use destack_core::fnv1a_128;
use serde::{Deserialize, Serialize};

/// Stable identifier for one canonical platform capability name.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlatformCapabilityId(
    /// Stable hash of the canonical capability string.
    pub u128,
);

impl PlatformCapabilityId {
    /// Build one capability identifier from one capability name.
    pub const fn from_name(name: &str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}
