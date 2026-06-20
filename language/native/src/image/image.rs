use serde::{Deserialize, Serialize};

/// In-memory native machine image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {}

impl Image {
    /// Create an empty native machine image.
    pub const fn empty() -> Self {
        Self {}
    }
}
