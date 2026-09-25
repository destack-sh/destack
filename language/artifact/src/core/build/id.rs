use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

const BUILD_ID_BYTES: usize = 16;

/// One TS++ toolchain build.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[repr(transparent)]
pub struct BuildId([u8; BUILD_ID_BYTES]);

impl BuildId {
    /// Create an id from its canonical bytes.
    pub const fn new(bytes: [u8; BUILD_ID_BYTES]) -> Self {
        Self(bytes)
    }

    /// Return the shared build id for isolated runs.
    pub const fn test() -> Self {
        Self([0x74; BUILD_ID_BYTES])
    }

    /// Return the build id bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for BuildId {
    /// Format the build id as lowercase hexadecimal.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }

        Ok(())
    }
}
