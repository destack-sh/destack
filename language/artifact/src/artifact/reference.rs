use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{ArtifactKey, ArtifactVersion};

/// Stable reference to one artifact payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ArtifactReference {
    /// Artifact key.
    pub key: ArtifactKey,
    /// Artifact version.
    pub version: ArtifactVersion,
}
