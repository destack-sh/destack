use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::ArtifactKey;

/// One provider failure that prevented an artifact payload from being published.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum ArtifactFailure {
    /// The provider produced user diagnostics without a payload.
    Diagnostics,
    /// One required artifact reached a non-ready terminal state.
    Requirement {
        /// The failed requirement key.
        key: ArtifactKey,
    },
}

impl ArtifactFailure {
    /// Build one diagnostic failure.
    pub fn diagnostics() -> Self {
        Self::Diagnostics
    }

    /// Build one requirement failure.
    pub fn requirement(key: ArtifactKey) -> Self {
        Self::Requirement { key }
    }
}
