use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use crate::{ArtifactKey, ArtifactVersion};

/// Stable reference to one artifact payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct ArtifactReference {
    /// Artifact key.
    pub key: ArtifactKey,
    /// Artifact version.
    pub version: ArtifactVersion,
}
