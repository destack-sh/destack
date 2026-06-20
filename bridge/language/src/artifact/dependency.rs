use destack_artifact as artifact;

use crate::{ArtifactVersion, ContentId, FileId, bridge};

/// One exact dependency read while building an artifact.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactDependency {
    /// Another exact artifact version.
    Artifact {
        /// The exact artifact version depended on.
        version: ArtifactVersion,
    },
    /// One exact primitive source observation.
    Source {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: ContentId,
    },
}

impl ArtifactDependency {
    /// Convert one public artifact dependency into one bridge dependency.
    pub(crate) fn from_artifact(dependency: artifact::ArtifactDependency) -> Option<Self> {
        match dependency {
            artifact::ArtifactDependency::Artifact(version) => Some(Self::Artifact {
                version: ArtifactVersion::from_artifact(version),
            }),
            artifact::ArtifactDependency::Projection(_) => None,
            artifact::ArtifactDependency::Source(dependency) => Some(Self::Source {
                file: FileId::from_source(dependency.file),
                content: ContentId::from_source(dependency.content),
            }),
        }
    }
}
