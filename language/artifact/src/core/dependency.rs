use serde::{Deserialize, Serialize};

use destack_source::{FileContentId, FileId};

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
    /// The entry path identity.
    pub path: FileId,
    /// The exact entry path state.
    pub state: ArtifactPathState,
}

impl ArtifactDirectoryEntry {
    /// Build one exact directory entry dependency.
    pub const fn new(path: FileId, state: ArtifactPathState) -> Self {
        Self { path, state }
    }
}

/// One primitive source observation read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceDependency {
    /// The exact state observed for one source path.
    PathState {
        /// The source path identity.
        path: FileId,
        /// The exact path state.
        state: ArtifactPathState,
    },
    /// The exact direct entries observed for one directory.
    DirectoryEntries {
        /// The source directory path identity.
        directory: FileId,
        /// The direct entries in deterministic order.
        entries: Vec<ArtifactDirectoryEntry>,
    },
    /// The exact source content read for one file.
    FileContent {
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: FileContentId,
    },
}

impl SourceDependency {
    /// Build one source path state dependency.
    pub fn path_state(path: FileId, state: ArtifactPathState) -> Self {
        Self::PathState { path, state }
    }

    /// Build one directory entries dependency.
    pub fn directory_entries(
        directory: FileId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) -> Self {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_unstable();
        entries.dedup();

        Self::DirectoryEntries { directory, entries }
    }

    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self::FileContent { file, content }
    }
}

/// Every dependency one artifact declares before it is built.
///
/// A provider collects this up front, so the closure is frozen before
/// the payload runs and the version can be fingerprinted without it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactDependencySet {
    /// Lower artifacts that must be resolved into versions first.
    pub artifacts: Vec<ArtifactKey>,
    /// Primitive source observations that feed the fingerprint.
    pub sources: Vec<SourceDependency>,
}

impl ArtifactDependencySet {
    /// Declare one required lower artifact.
    pub fn require(&mut self, key: ArtifactKey) {
        self.artifacts.push(key);
    }

    /// Declare one primitive source observation.
    pub fn observe(&mut self, source: SourceDependency) {
        self.sources.push(source);
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactDependency {
    /// Another exact artifact version.
    Artifact(ArtifactVersion),
    /// One exact primitive source observation.
    Source(SourceDependency),
}

impl ArtifactDependency {
    /// Build one artifact dependency.
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self::Artifact(version)
    }

    /// Build one source path state dependency.
    pub fn path_state(path: FileId, state: ArtifactPathState) -> Self {
        Self::Source(SourceDependency::path_state(path, state))
    }

    /// Build one directory entries dependency.
    pub fn directory_entries(
        directory: FileId,
        entries: impl IntoIterator<Item = ArtifactDirectoryEntry>,
    ) -> Self {
        Self::Source(SourceDependency::directory_entries(directory, entries))
    }

    /// Build one file content dependency.
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self::Source(SourceDependency::file_content(file, content))
    }
}
