use std::hash::Hash;

use destack_core::StableHasher;
use destack_source::{ComponentId, ContentId, FileId, ModuleId, StringId};
use serde::{Deserialize, Serialize};

use crate::{ArtifactKey, ArtifactVersion};

/// Exact source path state observed by one artifact computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactPathState {
    /// The path did not exist.
    Missing,
    /// The path was a regular file.
    File,
    /// The path was a directory.
    Directory,
    /// The path was a symbolic link.
    Symlink,
    /// The path existed with another host-specific kind.
    Other,
}

/// One exact directory entry observed by one artifact computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactDirectoryEntry {
    /// The entry path.
    pub path: StringId,
    /// The exact entry path state.
    pub state: ArtifactPathState,
}

impl ArtifactDirectoryEntry {
    /// Build one exact directory entry dependency.
    pub const fn new(path: StringId, state: ArtifactPathState) -> Self {
        Self { path, state }
    }
}

/// One primitive source observation read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceDependency {
    /// The exact state observed for one source path.
    PathState {
        /// The logical path.
        path: StringId,
        /// The exact path state.
        state: ArtifactPathState,
    },
    /// The exact direct entries observed for one directory.
    DirectoryEntries {
        /// The directory logical path.
        directory: StringId,
        /// The direct entries in deterministic order.
        entries: Vec<ArtifactDirectoryEntry>,
    },
    /// The exact source content read for one file.
    FileContent {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: ContentId,
    },
}

/// The identity of one primitive source observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceKey {
    /// The exact state of one source path.
    PathState(StringId),
    /// The exact direct entries of one source directory.
    DirectoryEntries(StringId),
    /// The exact content of one source file.
    FileContent(FileId),
}

/// Stable fingerprint of one observed artifact projection.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactProjectionFingerprint(u128);

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
    pub fn from_serialized_payload<T: Serialize>(value: &T) -> Result<Self, postcard::Error> {
        let bytes = postcard::to_allocvec(value)?;
        let mut hasher = StableHasher::new();

        hasher.update_len_prefixed(b"destack.artifact.projection.payload.v1");
        hasher.update_len_prefixed(&bytes);

        Ok(Self(hasher.finish_u128()))
    }
}

/// One observable projection of a component graph artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ComponentGraphProjection {
    /// The component containing one module.
    ComponentOf(ModuleId),
    /// The component and entry containing one module.
    ComponentEntryOf(ModuleId),
    /// The sorted modules belonging to one component.
    Members(ComponentId),
    /// The direct external components one component depends on.
    Dependencies(ComponentId),
}

/// One observable projection of an artifact payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactProjectionKey {
    /// A component graph projection.
    ComponentGraph(ComponentGraphProjection),
    /// A checked DIR module inside a checked component.
    DirChecked(ModuleId),
}

impl From<ComponentGraphProjection> for ArtifactProjectionKey {
    fn from(projection: ComponentGraphProjection) -> Self {
        Self::ComponentGraph(projection)
    }
}

/// One artifact projection selected by owner artifact and projection key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactProjection {
    /// The artifact that owns the projected payload.
    pub artifact: ArtifactKey,
    /// The projected value inside the owning artifact.
    pub key: ArtifactProjectionKey,
}

impl ArtifactProjection {
    /// Build one artifact projection.
    pub fn new(artifact: ArtifactKey, key: impl Into<ArtifactProjectionKey>) -> Self {
        Self {
            artifact,
            key: key.into(),
        }
    }
}

/// One exact projected artifact value observed while building an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactProjectionDependency {
    /// The projected artifact value.
    pub projection: ArtifactProjection,
    /// The exact projection fingerprint read from the owner artifact.
    pub fingerprint: ArtifactProjectionFingerprint,
}

impl ArtifactProjectionDependency {
    /// Build one exact artifact projection dependency.
    pub const fn new(
        projection: ArtifactProjection,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self {
            projection,
            fingerprint,
        }
    }
}

/// One unresolved artifact value needed before an artifact can be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactRequirement {
    /// The exact version of one artifact.
    Key(ArtifactKey),
    /// The exact fingerprint of one projected artifact value.
    Projection(ArtifactProjection),
}

impl ArtifactRequirement {
    /// Build one artifact version requirement.
    pub const fn version(key: ArtifactKey) -> Self {
        Self::Key(key)
    }

    /// Build one projected artifact value requirement.
    pub const fn projection(projection: ArtifactProjection) -> Self {
        Self::Projection(projection)
    }

    /// Return the artifact key this requirement resolves through.
    pub const fn artifact_key(&self) -> ArtifactKey {
        match self {
            Self::Key(key) => *key,
            Self::Projection(projection) => projection.artifact,
        }
    }
}

impl SourceDependency {
    /// Build one source path state dependency.
    pub fn path_state(path: StringId, state: ArtifactPathState) -> Self {
        Self::PathState { path, state }
    }

    /// Build one directory entries dependency.
    pub fn directory_entries(
        directory: StringId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) -> Self {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_unstable();
        entries.dedup();

        Self::DirectoryEntries { directory, entries }
    }

    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: ContentId) -> Self {
        Self::FileContent { file, content }
    }

    /// Return the value-independent source observation identity.
    pub const fn key(&self) -> SourceKey {
        match self {
            Self::PathState { path, .. } => SourceKey::PathState(*path),
            Self::DirectoryEntries { directory, .. } => SourceKey::DirectoryEntries(*directory),
            Self::FileContent { file, .. } => SourceKey::FileContent(*file),
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
        self.requirements.push(ArtifactRequirement::version(key));
    }

    /// Declare one required artifact projection.
    pub fn project(&mut self, artifact: ArtifactKey, key: impl Into<ArtifactProjectionKey>) {
        let projection = ArtifactProjection::new(artifact, key);

        self.requirements
            .push(ArtifactRequirement::projection(projection));
    }

    /// Declare one raw source observation.
    pub fn observe(&mut self, source: SourceDependency) {
        self.observe_source(source);
    }

    /// Declare one raw source observation.
    pub fn observe_source(&mut self, source: SourceDependency) {
        self.sources.push(source);
    }

    /// Declare one observed source path state.
    pub fn observe_path_state(&mut self, path: StringId, state: ArtifactPathState) {
        self.observe_source(SourceDependency::path_state(path, state));
    }

    /// Declare one observed source directory listing.
    pub fn observe_directory_entries(
        &mut self,
        directory: StringId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) {
        self.observe_source(SourceDependency::directory_entries(directory, entries));
    }

    /// Declare one observed source file content id.
    pub fn observe_file_content(&mut self, file: FileId, content: ContentId) {
        self.observe_source(SourceDependency::file_content(file, content));
    }

    /// Declare one observed regular source file.
    pub fn observe_file(&mut self, file: FileId, content: ContentId) {
        self.observe_file_content(file, content);
    }

    /// Mark the closure incomplete so the engine runs the collect pass again.
    pub fn mark_partial(&mut self) {
        self.is_partial = true;
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
        projection: ArtifactProjection,
        fingerprint: ArtifactProjectionFingerprint,
    ) -> Self {
        Self::Projection(ArtifactProjectionDependency::new(projection, fingerprint))
    }

    /// Build one source path state dependency.
    pub fn path_state(path: StringId, state: ArtifactPathState) -> Self {
        Self::Source(SourceDependency::path_state(path, state))
    }

    /// Build one directory entries dependency.
    pub fn directory_entries(
        directory: StringId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) -> Self {
        Self::Source(SourceDependency::directory_entries(directory, entries))
    }

    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: ContentId) -> Self {
        Self::Source(SourceDependency::file_content(file, content))
    }
}
