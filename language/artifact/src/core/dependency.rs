use std::fmt;
use std::hash::{Hash, Hasher};

use siphasher::sip128::Hasher128;
use tspp_core::{BlobId, StableHasher};
use tspp_dir::GlobalSymbolId;
use tspp_mir::Symbol;
use tspp_serde as serde;
use tspp_serde::Reflect;
use tspp_source::{FileId, ModuleId, PackageId};

use crate::{ArtifactKey, ArtifactVersion};

use ::serde::{Deserialize, Serialize};

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
        hasher.update_len_prefixed(b"tspp.artifact.modules.v1");
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
        hasher.update_len_prefixed(b"tspp.artifact.packages.v1");
        packages.hash(&mut hasher);

        Self(hasher.finish_u128())
    }
}

/// One exact repository source observation used by an artifact.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum SourceDependency {
    /// One source file and its exact bytes.
    File { file: FileId, blob: BlobId },
    /// One package configuration and its resolved dependencies.
    Package {
        package: PackageId,
        fingerprint: u128,
    },
    /// One module's contributing files and loader configuration.
    Module { module: ModuleId, fingerprint: u128 },
    /// The complete repository package set.
    Packages { fingerprint: PackageSetFingerprint },
    /// The complete repository module set.
    Modules { fingerprint: ModuleSetFingerprint },
    /// The module set of one package.
    PackageModules {
        package: PackageId,
        fingerprint: ModuleSetFingerprint,
    },
    /// The module resolution of one probed path.
    ModulePath { file: FileId, fingerprint: u128 },
}

/// One repository source selected by a source dependency.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum SourceDependencyKey {
    /// One source file's content.
    File(FileId),
    /// One package configuration and its resolved dependencies.
    Package(PackageId),
    /// One module's contributing files and loader configuration.
    Module(ModuleId),
    /// The repository package set.
    Packages,
    /// The repository module set.
    Modules,
    /// The module set of one package.
    PackageModules(PackageId),
    /// The module resolution of one probed path.
    ModulePath(FileId),
}

/// Stable fingerprint of one observed artifact projection.
#[repr(transparent)]
#[derive(
    Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactProjectionFingerprint(pub u128);

impl fmt::Debug for ArtifactProjectionFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "p{:032x}", self.0)
    }
}

impl ArtifactProjectionFingerprint {
    /// Build one artifact projection fingerprint from a stable value.
    pub fn new<T: Hash + ?Sized>(value: &T) -> Self {
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"tspp.artifact.projection.v1");
        value.hash(&mut hasher);

        Self(hasher.finish_u128())
    }

    /// Build one artifact projection fingerprint from a serialized artifact payload value.
    pub fn from_serialized_payload<T: Serialize>(value: &T) -> Result<Self, serde::Error> {
        // stream the canonical encoding straight into the hasher
        let mut hasher = siphasher::sip128::SipHasher13::new();
        Hasher::write(&mut hasher, b"tspp.artifact.projection.payload.v2");
        serde::hash_into(value, &mut hasher)?;
        let hash = hasher.finish128();

        Ok(Self(hash.as_u128()))
    }
}

/// One observable projection of an artifact payload.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ArtifactProjectionKey {
    /// The complete payload, independent of the inputs that built it.
    Payload,
    /// The sorted modules of one module graph.
    ModuleGraphModules,
    /// The import edges of one module in the module graph.
    ModuleGraphEdges(ModuleId),
    /// The implementations of one interface resolved across the module graph.
    ModuleGraphImplementations(GlobalSymbolId),
    /// The memory, execution, and parameter escape effects of one program function.
    ProgramAnalysisFunctionEffects(Symbol),
    /// The resolved relationships that shape the component graph.
    DirResolvedComponentRelations,
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
    /// The artifact owning the projected value.
    artifact: ArtifactKey,
    /// The projected value inside the artifact.
    key: ArtifactProjectionKey,
    /// The exact projection fingerprint read from the owner artifact.
    fingerprint: ArtifactProjectionFingerprint,
}

impl ArtifactProjectionDependency {
    /// Build one exact artifact projection dependency.
    pub const fn new(
        artifact: ArtifactKey,
        key: ArtifactProjectionKey,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self {
            artifact,
            key,
            fingerprint,
        }
    }

    /// Return the observed artifact projection.
    pub const fn projection(self) -> ArtifactProjection {
        ArtifactProjection::new(self.artifact, self.key)
    }

    /// Return the exact observed projection fingerprint.
    pub const fn fingerprint(self) -> ArtifactProjectionFingerprint {
        self.fingerprint
    }
}

/// One unresolved artifact value needed before an artifact can be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactRequirement {
    /// One complete artifact result.
    Artifact(ArtifactKey),
    /// One projected artifact value.
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
    /// Build one file dependency.
    pub const fn file(file: FileId, blob: BlobId) -> Self {
        Self::File { file, blob }
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

    /// Build one package module set dependency.
    pub fn package_modules(package: PackageId, modules: &[ModuleId]) -> Self {
        Self::PackageModules {
            package,
            fingerprint: ModuleSetFingerprint::new(modules),
        }
    }

    /// Build one module path probe dependency.
    pub fn module_path(file: FileId, module: Option<ModuleId>) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.artifact.module-path.v1");
        module.hash(&mut hasher);

        Self::ModulePath {
            file,
            fingerprint: hasher.finish_u128(),
        }
    }

    /// Return the repository source selected by this observation.
    pub const fn key(self) -> SourceDependencyKey {
        match self {
            Self::File { file, .. } => SourceDependencyKey::File(file),
            Self::Package { package, .. } => SourceDependencyKey::Package(package),
            Self::Module { module, .. } => SourceDependencyKey::Module(module),
            Self::Packages { .. } => SourceDependencyKey::Packages,
            Self::Modules { .. } => SourceDependencyKey::Modules,
            Self::PackageModules { package, .. } => SourceDependencyKey::PackageModules(package),
            Self::ModulePath { file, .. } => SourceDependencyKey::ModulePath(file),
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

    /// Declare one required artifact payload.
    pub fn require_payload(&mut self, artifact: ArtifactKey) {
        self.require_projection(artifact, ArtifactProjectionKey::Payload);
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
    pub fn observe_file(&mut self, file: FileId, blob: BlobId) {
        self.observe(SourceDependency::file(file, blob));
    }

    /// Declare the complete repository package identity set.
    pub fn observe_packages(&mut self, packages: &[PackageId]) {
        self.observe(SourceDependency::packages(packages));
    }

    /// Declare the complete repository module identity set.
    pub fn observe_modules(&mut self, modules: &[ModuleId]) {
        self.observe(SourceDependency::modules(modules));
    }

    /// Declare the module identity set of one package.
    pub fn observe_package_modules(&mut self, package: PackageId, modules: &[ModuleId]) {
        self.observe(SourceDependency::package_modules(package, modules));
    }

    /// Mark the closure incomplete so the engine runs the collect pass again.
    pub fn mark_partial(&mut self) {
        self.is_partial = true;
    }

    /// Sort and deduplicate the declared dependencies.
    pub fn normalize(&mut self) {
        self.requirements.sort_unstable();
        self.requirements.dedup();
        self.sources.sort_unstable();
        self.sources.dedup();
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

/// One changed observation propagated through artifact dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactInvalidation {
    /// One exact artifact result.
    Artifact(ArtifactVersion),
    /// The projected values of one artifact.
    Projections(ArtifactKey),
    /// One exact repository source observation.
    Source(SourceDependency),
}

impl ArtifactDependency {
    /// Build one artifact dependency.
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self::Artifact(version)
    }

    /// Build one exact artifact projection dependency.
    pub fn projection(
        artifact: ArtifactKey,
        key: ArtifactProjectionKey,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self::Projection(ArtifactProjectionDependency::new(
            artifact,
            key,
            fingerprint,
        ))
    }

    /// Return the depended-on artifact key, excluding primitive sources.
    pub const fn artifact_key(&self) -> Option<ArtifactKey> {
        match self {
            Self::Artifact(version) => Some(version.key),
            Self::Projection(dependency) => Some(dependency.artifact),
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

    /// Return the invalidation propagated by a changed dependency.
    pub const fn invalidation(&self) -> ArtifactInvalidation {
        match self {
            Self::Artifact(version) => ArtifactInvalidation::Artifact(*version),
            Self::Projection(projection) => ArtifactInvalidation::Projections(projection.artifact),
            Self::Source(source) => ArtifactInvalidation::Source(*source),
        }
    }
}
