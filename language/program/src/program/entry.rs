use serde::{Deserialize, Serialize};

/// One program entrypoint id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryPoint(u32);

impl EntryPoint {
    /// Create one program entrypoint.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the entrypoint index.
    pub const fn index(self) -> u32 {
        self.0
    }
}
