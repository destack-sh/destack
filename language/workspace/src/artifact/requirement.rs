use super::{ArtifactDependency, ArtifactKey};

/// Requirement for one semantic artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactRequirement {
    /// The required artifact key.
    pub key: ArtifactKey,
    /// The expected dependency for that key.
    pub dependency: ArtifactDependency,
}

impl ArtifactRequirement {
    /// Create a new artifact requirement.
    pub fn new(key: ArtifactKey, dependency: ArtifactDependency) -> Self {
        Self { key, dependency }
    }
}
