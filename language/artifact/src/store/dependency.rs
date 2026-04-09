use serde::{Deserialize, Serialize};

use crate::ArtifactVersion;

/// Live dependency proof for one published artifact version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactDependency {
    /// The exact dependency version used while building the artifact.
    pub version: ArtifactVersion,
}
