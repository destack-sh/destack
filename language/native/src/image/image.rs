use serde::{Deserialize, Serialize};

use crate::Continuation;

/// In-memory native executor image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    /// The suspended continuation when execution is currently paused.
    pub continuation: Option<Continuation>,
}

impl Image {
    /// Create an empty native executor image.
    pub const fn empty() -> Self {
        Self { continuation: None }
    }
}
