use serde::{Deserialize, Serialize};

use crate::{ArtifactKey, ArtifactVersion};

/// One provider failure that prevented an artifact payload from being published.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactFailure {
    /// The provider produced user diagnostics without a payload.
    Diagnostics,
    /// One required artifact reached a non-ready terminal state.
    Requirement {
        /// The failed requirement key.
        key: ArtifactKey,
    },
    /// The artifact store returned an invalid payload boundary.
    Corrupt {
        /// The corrupt exact artifact version.
        version: ArtifactVersion,
    },
    /// The provider failed due to infrastructure or toolchain state.
    Internal {
        /// The failure message.
        message: String,
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

    /// Build one corrupt artifact boundary failure.
    pub fn corrupt(version: ArtifactVersion) -> Self {
        Self::Corrupt { version }
    }

    /// Build one internal provider failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}
