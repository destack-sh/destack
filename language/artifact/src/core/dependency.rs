use std::hash::Hash;

use destack_core::StableHasher;
use destack_serde::Reflect;
use destack_source::{ComponentId, ContentId, FileId, ModuleId, PackageId};
use serde::{Deserialize, Serialize};
use siphasher::sip128::Hasher128;

use crate::{ArtifactKey, ArtifactVersion};

/// Stable fingerprint of one repository module set.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ModuleSetFingerprint(u128);

impl ModuleSetFingerprint {
    /// Build one fingerprint from module identities in stable order.
    pub fn new(modules: &[ModuleId]) -> Self {
        let mut modules = modules.to_vec();
        modules.sort_unstable();
        modules.dedup();

        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.artifact.modules.v1");
        modules.hash(&mut hasher);

        Self(hasher.finish_u128())
    }
}

/// Stable fingerprint of one repository package set.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct PackageSetFingerprint(u128);

impl PackageSetFingerprint {
    /// Build one fingerprint from package identities in stable order.
    pub fn new(packages: &[PackageId]) -> Self {
        let mut packages = packages.to_vec();
        packages.sort_unstable();
        packages.dedup();

        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"destack.artifact.packages.v1");
        packages.hash(&mut hasher);

        Self(hasher.finish_u128())
    }
}

/// One exact repository source observation used by an artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum SourceDependency {
    /// One exact source file content.
    FileContent {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: ContentId,
    },
    /// The complete repository package identity set.
    Packages {
        /// The observed package set fingerprint.
        fingerprint: PackageSetFingerprint,
    },
    /// The complete repository module identity set.
    Modules {
        /// The observed module set fingerprint.
        fingerprint: ModuleSetFingerprint,
    },
}

/// One repository source selected by a source dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum SourceDependencyKey {
    /// One source file.
    File(FileId),
    /// The repository package set.
    Packages,
    /// The repository module set.
    Modules,
}

/// Stable fingerprint of one observed artifact projection.
#[repr(transparent)]
#[derive(
    Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactProjectionFingerprint(pub u128);

impl std::fmt::Debug for ArtifactProjectionFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "p{:032x}", self.0)
    }
}

impl ArtifactProjectionFingerprint {
    /// Build one artifact projection fingerprint from a stable value.
    pub fn new<T: Hash + ?Sized>(value: &T) -> Self {
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"destack.artifact.projection.v1");
        value.hash(&mut hasher);

        Self(hasher.finish_u128())
    }

    /// Build one artifact projection fingerprint from a serialized artifact payload value.
    pub fn from_serialized_payload<T: Serialize>(value: &T) -> Result<Self, destack_serde::Error> {
        // stream the canonical encoding straight into the hasher
        let mut hasher = siphasher::sip128::SipHasher13::new();
        std::hash::Hasher::write(&mut hasher, b"destack.artifact.projection.payload.v2");
        destack_serde::hash_into(value, &mut hasher)?;
        let hash = hasher.finish128();

        Ok(Self(hash.as_u128()))
    }
}

/// One observable projection of an artifact payload.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ArtifactProjectionKey {
    /// The reference component containing one module.
    ReferenceComponent(ModuleId),
    /// The sorted modules belonging to one reference component.
    ReferenceMembers(ComponentId),
    /// The direct external reference components one component depends on.
    ReferenceDependencies(ComponentId),
    /// The inference component containing one module.
    InferenceComponent(ModuleId),
    /// The sorted inference component identities.
    InferenceComponents,
    /// The member modules of one inference component.
    InferenceMembers(ComponentId),
    /// The upstream inference components one inference component depends on.
    InferenceDependencies(ComponentId),
    /// The inherent extensions resolved across the component graph.
    InherentExtensions,
    /// A declared DIR module inside a declared component.
    DirDeclaredModule(ModuleId),
    /// Exported symbols whose consumers require inference.
    DirInferenceExports,
    /// Resolved relationships that determine component graph edges.
    DirComponentEdges,
    /// A checked DIR module inside a checked component.
    DirCheckedModule(ModuleId),
}

/// One artifact projection selected by owner artifact and projection key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactProjection {
    /// The artifact that owns the projected payload.
    pub artifact: ArtifactKey,
    /// The projected value inside the owning artifact.
    pub key: ArtifactProjectionKey,
}

impl ArtifactProjection {
    /// Build one artifact projection.
    pub const fn new(artifact: ArtifactKey, key: ArtifactProjectionKey) -> Self {
        Self { artifact, key }
    }
}

/// One exact projected artifact value observed while building an artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactProjectionDependency {
    /// The exact artifact version that supplied the projected value.
    version: ArtifactVersion,
    /// The projected artifact value.
    projection: ArtifactProjection,
    /// The exact projection fingerprint read from the owner artifact.
    fingerprint: ArtifactProjectionFingerprint,
}

impl ArtifactProjectionDependency {
    /// Build one exact artifact projection dependency.
    pub const fn new(
        version: ArtifactVersion,
        key: ArtifactProjectionKey,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self {
            version,
            projection: ArtifactProjection::new(version.key, key),
            fingerprint,
        }
    }

    /// Return the exact artifact version.
    pub const fn version(self) -> ArtifactVersion {
        self.version
    }

    /// Return the observed artifact projection.
    pub const fn projection(self) -> ArtifactProjection {
        self.projection
    }

    /// Return the exact observed projection fingerprint.
    pub const fn fingerprint(self) -> ArtifactProjectionFingerprint {
        self.fingerprint
    }

    /// Return whether the artifact version owns this projection.
    pub(crate) fn owner_matches_version(&self) -> bool {
        self.version.key == self.projection.artifact
    }
}

/// One unresolved artifact value needed before an artifact can be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactRequirement {
    /// One complete artifact result.
    Artifact(ArtifactKey),
    /// The exact fingerprint of one projected artifact value.
    Projection(ArtifactProjection),
}

impl ArtifactRequirement {
    /// Build one complete artifact requirement.
    pub const fn artifact(key: ArtifactKey) -> Self {
        Self::Artifact(key)
    }

    /// Build one projected artifact value requirement.
    pub const fn projection(projection: ArtifactProjection) -> Self {
        Self::Projection(projection)
    }

    /// Return the artifact key this requirement resolves through.
    pub const fn artifact_key(&self) -> ArtifactKey {
        match self {
            Self::Artifact(key) => *key,
            Self::Projection(projection) => projection.artifact,
        }
    }
}

impl SourceDependency {
    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: ContentId) -> Self {
        Self::FileContent { file, content }
    }

    /// Build one complete package set dependency.
    pub fn packages(packages: &[PackageId]) -> Self {
        Self::Packages {
            fingerprint: PackageSetFingerprint::new(packages),
        }
    }

    /// Build one complete module set dependency.
    pub fn modules(modules: &[ModuleId]) -> Self {
        Self::Modules {
            fingerprint: ModuleSetFingerprint::new(modules),
        }
    }

    /// Return the selected repository source.
    const fn key(self) -> SourceDependencyKey {
        match self {
            Self::FileContent { file, .. } => SourceDependencyKey::File(file),
            Self::Packages { .. } => SourceDependencyKey::Packages,
            Self::Modules { .. } => SourceDependencyKey::Modules,
        }
    }
}

/// Every dependency one artifact declares before it is built.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactDependencySet {
    /// Artifact values that must be resolved first.
    pub requirements: Vec<ArtifactRequirement>,
    /// Primitive source observations that feed the fingerprint.
    pub sources: Vec<SourceDependency>,
    /// Whether the collect pass stopped before naming the full closure.
    pub is_partial: bool,
}

impl ArtifactDependencySet {
    /// Declare one required lower artifact.
    pub fn require(&mut self, key: ArtifactKey) {
        self.requirements.push(ArtifactRequirement::artifact(key));
    }

    /// Declare one required artifact projection.
    pub fn require_projection(&mut self, artifact: ArtifactKey, key: ArtifactProjectionKey) {
        let projection = ArtifactProjection::new(artifact, key);

        self.requirements
            .push(ArtifactRequirement::projection(projection));
    }

    /// Declare one source file content dependency.
    pub fn observe(&mut self, source: SourceDependency) {
        self.sources.push(source);
    }

    /// Declare one observed regular source file.
    pub fn observe_file(&mut self, file: FileId, content: ContentId) {
        self.observe(SourceDependency::file_content(file, content));
    }

    /// Declare the complete repository package identity set.
    pub fn observe_packages(&mut self, packages: &[PackageId]) {
        self.observe(SourceDependency::packages(packages));
    }

    /// Declare the complete repository module identity set.
    pub fn observe_modules(&mut self, modules: &[ModuleId]) {
        self.observe(SourceDependency::modules(modules));
    }

    /// Mark the closure incomplete so the engine runs the collect pass again.
    pub fn mark_partial(&mut self) {
        self.is_partial = true;
    }

    /// Return whether dependencies follow this set's requirement order.
    pub fn matches(&self, dependencies: &[ArtifactDependency]) -> bool {
        if self.is_partial || dependencies.len() != self.requirements.len() + self.sources.len() {
            return false;
        }

        // match artifact declarations by requirement
        let (requirements, sources) = dependencies.split_at(self.requirements.len());
        let is_requirements_match = self
            .requirements
            .iter()
            .zip(requirements)
            .all(|(requirement, dependency)| dependency.requirement() == Some(*requirement));

        // match source declarations without comparing observed values
        let is_sources_match = self
            .sources
            .iter()
            .zip(sources)
            .all(|(source, dependency)| match dependency {
                ArtifactDependency::Source(dependency) => source.key() == dependency.key(),
                ArtifactDependency::Artifact(_) | ArtifactDependency::Projection(_) => false,
            });

        is_requirements_match && is_sources_match
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum ArtifactDependency {
    /// Another exact artifact version.
    Artifact(ArtifactVersion),
    /// One exact projected artifact value.
    Projection(ArtifactProjectionDependency),
    /// One exact primitive source observation.
    Source(SourceDependency),
}

impl ArtifactDependency {
    /// Build one artifact dependency.
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self::Artifact(version)
    }

    /// Build one exact artifact projection dependency.
    pub fn projection(
        version: ArtifactVersion,
        key: ArtifactProjectionKey,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self::Projection(ArtifactProjectionDependency::new(version, key, fingerprint))
    }

    /// Return the depended-on artifact key, excluding primitive sources.
    pub const fn artifact_key(&self) -> Option<ArtifactKey> {
        match self {
            Self::Artifact(version) => Some(version.key),
            Self::Projection(dependency) => Some(dependency.version().key),
            Self::Source(_) => None,
        }
    }

    /// Return the exact artifact observation represented by this dependency.
    pub const fn requirement(&self) -> Option<ArtifactRequirement> {
        match self {
            Self::Artifact(version) => Some(ArtifactRequirement::Artifact(version.key)),
            Self::Projection(dependency) => {
                Some(ArtifactRequirement::Projection(dependency.projection()))
            }
            Self::Source(_) => None,
        }
    }
}
