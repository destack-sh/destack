// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, FileContentId, FileId};

/// One exact directory entry observed by one artifact computation.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactDirectoryEntry {
    /// The entry path identity.
    pub path: FileId,
    /// The exact entry path state.
    pub state: String,
}

impl ArtifactDirectoryEntry {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactDirectoryEntry) -> Self {
        Self {
            path: FileId::from_bridge(value.path),
            state: artifact_path_state_label(value.state),
        }
    }
}

/// One primitive source observation read while building an artifact.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactSourceDependency {
    /// Payload variant label.
    pub kind: String,
    /// The source path identity.
    pub path: Option<FileId>,
    /// The exact path state.
    pub state: Option<String>,
    /// The source directory path identity.
    pub directory: Option<FileId>,
    /// The direct entries in deterministic order.
    pub entries: Option<Vec<ArtifactDirectoryEntry>>,
    /// The source file id.
    pub file: Option<FileId>,
    /// The exact source content id.
    pub content: Option<FileContentId>,
}

impl ArtifactSourceDependency {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactSourceDependency) -> Self {
        match value {
            bridge::ArtifactSourceDependency::PathState { path, state } => Self {
                kind: "pathState".to_string(),
                path: Some(FileId::from_bridge(path)),
                state: Some(artifact_path_state_label(state)),
                directory: None,
                entries: None,
                file: None,
                content: None,
            },
            bridge::ArtifactSourceDependency::DirectoryEntries { directory, entries } => Self {
                kind: "directoryEntries".to_string(),
                directory: Some(FileId::from_bridge(directory)),
                entries: Some(
                    entries
                        .into_iter()
                        .map(|item| ArtifactDirectoryEntry::from_bridge(item))
                        .collect(),
                ),
                path: None,
                state: None,
                file: None,
                content: None,
            },
            bridge::ArtifactSourceDependency::FileContent { file, content } => Self {
                kind: "fileContent".to_string(),
                file: Some(FileId::from_bridge(file)),
                content: Some(FileContentId::from_bridge(content)),
                path: None,
                state: None,
                directory: None,
                entries: None,
            },
        }
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactDependency {
    /// Payload variant label.
    pub kind: String,
    /// The exact artifact version depended on.
    pub version: Option<ArtifactVersion>,
    /// The primitive source observation.
    pub dependency: Option<ArtifactSourceDependency>,
}

impl ArtifactDependency {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactDependency) -> Self {
        match value {
            bridge::ArtifactDependency::Artifact { version } => Self {
                kind: "artifact".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                dependency: None,
            },
            bridge::ArtifactDependency::Source { dependency } => Self {
                kind: "source".to_string(),
                dependency: Some(ArtifactSourceDependency::from_bridge(dependency)),
                version: None,
            },
        }
    }
}

/// Return one target enum label.
fn artifact_path_state_label(value: bridge::ArtifactPathState) -> String {
    let label = match value {
        bridge::ArtifactPathState::Missing => "missing",
        bridge::ArtifactPathState::File => "file",
        bridge::ArtifactPathState::Directory => "directory",
        bridge::ArtifactPathState::Symlink => "symlink",
        bridge::ArtifactPathState::Other => "other",
    };
    label.to_string()
}
