use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// User-facing runtime entrypoint name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Entry {
    /// The fully qualified entry name.
    name: String,
}

impl Entry {
    /// Create one runtime entrypoint name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Return the fully qualified entry name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
