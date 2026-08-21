use super::{ArtifactBindingId, ArtifactId};
use crate::{ArtifactDependency, ArtifactKey, ArtifactVersion, SourceDependency};

/// One value whose change may invalidate artifact bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactDependencyOwner {
    /// One exact artifact result.
    Artifact(ArtifactVersion),
    /// The projections of one artifact, whichever version owns them.
    Projection(ArtifactKey),
    /// One exact repository source observation.
    Source(SourceDependency),
}

impl From<&ArtifactDependency> for ArtifactDependencyOwner {
    /// Convert one exact dependency observation into its invalidation owner.
    fn from(dependency: &ArtifactDependency) -> Self {
        match dependency {
            ArtifactDependency::Artifact(version) => Self::Artifact(*version),
            ArtifactDependency::Projection(projection) => {
                Self::Projection(projection.projection().artifact)
            }
            ArtifactDependency::Source(source) => Self::Source(*source),
        }
    }
}

/// One immutable binding dependency indexed by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactDependent {
    /// The dependent artifact.
    pub artifact: ArtifactId,
    /// The dependent binding.
    pub binding: ArtifactBindingId,
    /// The dependency ordinal inside the binding.
    pub dependency: u32,
}
