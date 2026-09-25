use serde::{Deserialize, Serialize};
use tspp_core::fnv1a_128;

/// Default codec used for binding trace payloads.
pub const DEFAULT_BINDING_CODEC: CodecId = CodecId::from_name("destack-serde-v1");

/// Stable identifier for a binding trace codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CodecId(pub u128);

impl CodecId {
    /// Build a codec id from a static codec name.
    pub const fn from_name(name: &'static str) -> Self {
        Self(fnv1a_128(name.as_bytes()))
    }
}
