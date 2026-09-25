use std::hash::Hash;

use serde::{Deserialize, Serialize};
use tspp_core::StableHasher;
use tspp_serde::Reflect;

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactProjection, ArtifactProjectionFingerprint,
    ArtifactVersion, BuildId, SourceDependency,
};

/// Deterministic fingerprint of one artifact's observed inputs.
#[repr(transparent)]
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactFingerprint(pub u128);

impl std::fmt::Debug for ArtifactFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "f{:032x}", self.0)
    }
}

impl std::fmt::Display for ArtifactFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "f{:032x}", self.0)
    }
}

impl ArtifactFingerprint {
    /// Create one artifact fingerprint from the key, build, and dependency observations.
    pub(crate) fn new(
        key: ArtifactKey,
        build_id: BuildId,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        // artifact dependency identities are a set
        let mut dependencies = dependencies
            .into_iter()
            .map(ArtifactFingerprintDependency::from)
            .collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies.dedup();

        // stable fingerprint stream
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"tspp.artifact.inputs.v3");
        key.hash(&mut hasher);
        hasher.update(build_id.as_bytes());
        hasher.update(&(dependencies.len() as u64).to_le_bytes());
        for dependency in &dependencies {
            dependency.hash(&mut hasher);
        }

        Self(hasher.finish_u128())
    }
}

/// One dependency identity included in an artifact fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ArtifactFingerprintDependency {
    /// Another exact artifact version.
    Artifact(ArtifactVersion),
    /// One exact projected artifact value.
    Projection {
        /// The projected artifact value.
        projection: ArtifactProjection,
        /// The exact projection fingerprint.
        fingerprint: ArtifactProjectionFingerprint,
    },
    /// One exact primitive source observation.
    Source(SourceDependency),
}

impl From<ArtifactDependency> for ArtifactFingerprintDependency {
    fn from(dependency: ArtifactDependency) -> Self {
        match dependency {
            ArtifactDependency::Artifact(version) => Self::Artifact(version),
            ArtifactDependency::Projection(dependency) => Self::Projection {
                projection: dependency.projection(),
                fingerprint: dependency.fingerprint(),
            },
            ArtifactDependency::Source(dependency) => Self::Source(dependency),
        }
    }
}
