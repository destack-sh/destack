use destack_artifact as artifact;
use destack_core::StringPool;

use crate::{ArtifactVersion, ContentId, FileId, bridge};

/// Exact source path state observed by one artifact computation.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactDirectoryEntry {
    /// The entry path.
    pub path: String,
    /// The exact entry path state.
    pub state: ArtifactPathState,
}

/// One primitive source observation read while building an artifact.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactSourceDependency {
    /// The exact state observed for one source path.
    PathState {
        /// The source path.
        path: String,
        /// The exact path state.
        state: ArtifactPathState,
    },
    /// The exact direct entries observed for one directory.
    DirectoryEntries {
        /// The source directory path.
        directory: String,
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
        /// The primitive source observation.
        dependency: ArtifactSourceDependency,
    },
}

impl ArtifactPathState {
    /// Convert one artifact path state into one bridge path state.
    pub fn from_artifact(state: artifact::ArtifactPathState) -> Self {
        match state {
            artifact::ArtifactPathState::Missing => Self::Missing,
            artifact::ArtifactPathState::File => Self::File,
            artifact::ArtifactPathState::Directory => Self::Directory,
            artifact::ArtifactPathState::Symlink => Self::Symlink,
            artifact::ArtifactPathState::Other => Self::Other,
        }
    }
}

impl ArtifactDirectoryEntry {
    /// Convert one artifact directory entry into one bridge directory entry.
    pub fn from_artifact(entry: artifact::ArtifactDirectoryEntry, strings: &StringPool) -> Self {
        Self {
            path: strings.get(entry.path).to_string(),
            state: ArtifactPathState::from_artifact(entry.state),
        }
    }
}

impl ArtifactSourceDependency {
    /// Convert one artifact source dependency into one bridge source dependency.
    pub fn from_artifact(dependency: artifact::SourceDependency, strings: &StringPool) -> Self {
        match dependency {
            artifact::SourceDependency::PathState { path, state } => Self::PathState {
                path: strings.get(path).to_string(),
                state: ArtifactPathState::from_artifact(state),
            },
            artifact::SourceDependency::DirectoryEntries { directory, entries } => {
                let entries = entries
                    .into_iter()
                    .map(|entry| ArtifactDirectoryEntry::from_artifact(entry, strings))
                    .collect();

                Self::DirectoryEntries {
                    directory: strings.get(directory).to_string(),
                    entries,
                }
            }
            artifact::SourceDependency::FileContent { file, content } => Self::FileContent {
                file: FileId::from_source(file),
                content: ContentId::from_source(content),
            },
        }
    }
}

impl ArtifactDependency {
    /// Convert one artifact dependency into one bridge dependency.
    pub fn from_artifact(dependency: artifact::ArtifactDependency, strings: &StringPool) -> Self {
        match dependency {
            artifact::ArtifactDependency::Artifact(version) => Self::Artifact {
                version: ArtifactVersion::from_artifact(version),
            },
            artifact::ArtifactDependency::Source(dependency) => Self::Source {
                dependency: ArtifactSourceDependency::from_artifact(dependency, strings),
            },
        }
    }
}

impl From<artifact::ArtifactPathState> for ArtifactPathState {
    /// Convert one artifact path state into one bridge path state.
    fn from(state: artifact::ArtifactPathState) -> Self {
        Self::from_artifact(state)
    }
}
