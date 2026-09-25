use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One exact RPC wire grammar.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ProtocolVersion(pub u16);

impl ProtocolVersion {
    /// Current RPC wire grammar.
    pub const CURRENT: Self = Self(1);

    /// Create one exact RPC wire grammar version.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Select the first preferred version supported by the peer.
    pub(crate) fn negotiate(preferred: &[Self], supported: &[Self]) -> Option<Self> {
        preferred
            .iter()
            .copied()
            .find(|version| supported.contains(version))
    }
}
