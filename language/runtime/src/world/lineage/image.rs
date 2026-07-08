use serde::{Deserialize, Serialize};

/// Image identifier for one materialized world state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ImageId(u64);

impl ImageId {
    /// Create a new image identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw image identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
