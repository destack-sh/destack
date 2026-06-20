use serde::{Deserialize, Serialize};

/// Current Destack native ABI version.
pub const NATIVE_ABI_VERSION: u32 = 1;

/// Native ABI required by one native code payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abi {
    /// Destack native ABI version.
    pub version: u32,
    /// Target triple or equivalent target identity.
    pub target: String,
    /// Native pointer byte width expected by this code.
    pub pointer_bytes: u8,
}

impl Abi {
    /// Create one native ABI descriptor.
    pub fn new(target: String, pointer_bytes: u8) -> Self {
        Self {
            version: NATIVE_ABI_VERSION,
            target,
            pointer_bytes,
        }
    }
}
