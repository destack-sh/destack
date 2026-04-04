use serde::{Deserialize, Serialize};

use crate::{ArtifactKey, ArtifactStamp};

/// Live dependency proof for one published artifact version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactDependency {
    /// The required artifact key.
    pub key: ArtifactKey,
    /// The exact dependency stamp used while building the artifact.
    pub stamp: ArtifactStamp,
}
