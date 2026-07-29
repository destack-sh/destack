use std::hash::Hash;

use destack_core::StableHasher;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactProjection, ArtifactProjectionFingerprint,
    ArtifactVersion, SourceDependency,
};

/// Deterministic fingerprint of one artifact's complete inputs.
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
    /// Create one artifact fingerprint from the toolchain build and exact dependencies.
    pub(crate) fn new(
        build_fingerprint: &str,
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

        hasher.update_len_prefixed(b"destack.artifact.inputs.v1");
        hasher.update_len_prefixed(build_fingerprint.as_bytes());
        hasher.update(&(dependencies.len() as u64).to_le_bytes());
        for dependency in &dependencies {
            dependency.hash(&mut hasher);
        }

        Self(hasher.finish_u128())
    }
}

/// One artifact key and its exact input fingerprint.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactInput {
    /// The artifact key.
    pub key: ArtifactKey,
    /// The exact input fingerprint.
    pub fingerprint: ArtifactFingerprint,
}

impl ArtifactInput {
    /// Create one artifact input from its complete declared dependencies.
    pub fn new(
        key: ArtifactKey,
        build_fingerprint: &str,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        let fingerprint = ArtifactFingerprint::new(build_fingerprint, dependencies);

        Self { key, fingerprint }
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
