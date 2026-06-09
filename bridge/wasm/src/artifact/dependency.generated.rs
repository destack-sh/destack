// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, FileContentId, FileId};

/// One exact directory entry observed by one artifact computation.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactDirectoryEntry {
    path: FileId,
    state: String,
}

#[wasm_bindgen]
impl ArtifactDirectoryEntry {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(path: FileId, state: String) -> Self {
        Self { path, state }
    }

    /// The entry path identity.
    #[wasm_bindgen(getter, js_name = "path")]
    pub fn path(&self) -> FileId {
        self.path.clone()
    }

    /// The exact entry path state.
    #[wasm_bindgen(getter, js_name = "state")]
    pub fn state(&self) -> String {
        self.state.clone()
    }
}

impl ArtifactDirectoryEntry {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::ArtifactDirectoryEntry) -> Self {
        Self {
            path: FileId::from_bridge(value.path),
            state: artifact_path_state_label(value.state),
        }
    }
}

/// One primitive source observation read while building an artifact.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactSourceDependency {
    content: ArtifactSourceDependencyContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum ArtifactSourceDependencyContent {
    /// The exact state observed for one source path.
    PathState {
        /// The source path identity.
        path: FileId,
        /// The exact path state.
        state: String,
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

#[wasm_bindgen]
impl ArtifactSourceDependency {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "pathState")]
    pub fn path_state(path: FileId, state: String) -> Self {
        Self {
            content: ArtifactSourceDependencyContent::PathState { path, state },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "directoryEntries")]
    pub fn directory_entries(directory: FileId, entries: Vec<ArtifactDirectoryEntry>) -> Self {
        Self {
            content: ArtifactSourceDependencyContent::DirectoryEntries { directory, entries },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "fileContent")]
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self {
            content: ArtifactSourceDependencyContent::FileContent { file, content },
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            ArtifactSourceDependencyContent::PathState { .. } => "pathState",
            ArtifactSourceDependencyContent::DirectoryEntries { .. } => "directoryEntries",
            ArtifactSourceDependencyContent::FileContent { .. } => "fileContent",
        };
        label.to_string()
    }

    /// The source path identity.
    #[wasm_bindgen(getter, js_name = "path")]
    pub fn path(&self) -> Option<FileId> {
        match &self.content {
            ArtifactSourceDependencyContent::PathState { path: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The exact path state.
    #[wasm_bindgen(getter, js_name = "state")]
    pub fn state(&self) -> Option<String> {
        match &self.content {
            ArtifactSourceDependencyContent::PathState { state: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The source directory path identity.
    #[wasm_bindgen(getter, js_name = "directory")]
    pub fn directory(&self) -> Option<FileId> {
        match &self.content {
            ArtifactSourceDependencyContent::DirectoryEntries {
                directory: value, ..
            } => Some(value.clone()),
            _ => None,
        }
    }

    /// The direct entries in deterministic order.
    #[wasm_bindgen(getter, js_name = "entries")]
    pub fn entries(&self) -> Option<Vec<ArtifactDirectoryEntry>> {
        match &self.content {
            ArtifactSourceDependencyContent::DirectoryEntries { entries: value, .. } => {
                Some(value.clone())
            }
            _ => None,
        }
    }

    /// The source file id.
    #[wasm_bindgen(getter, js_name = "file")]
    pub fn file(&self) -> Option<FileId> {
        match &self.content {
            ArtifactSourceDependencyContent::FileContent { file: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The exact source content id.
    #[wasm_bindgen(getter, js_name = "content")]
    pub fn content(&self) -> Option<FileContentId> {
        match &self.content {
            ArtifactSourceDependencyContent::FileContent { content: value, .. } => {
                Some(value.clone())
            }
            _ => None,
        }
    }
}

impl ArtifactSourceDependency {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactSourceDependency) -> Self {
        match value {
            bridge::ArtifactSourceDependency::PathState { path, state } => Self {
                content: ArtifactSourceDependencyContent::PathState {
                    path: FileId::from_bridge(path),
                    state: artifact_path_state_label(state),
                },
            },
            bridge::ArtifactSourceDependency::DirectoryEntries { directory, entries } => Self {
                content: ArtifactSourceDependencyContent::DirectoryEntries {
                    directory: FileId::from_bridge(directory),
                    entries: entries
                        .into_iter()
                        .map(|item| ArtifactDirectoryEntry::from_bridge(item))
                        .collect(),
                },
            },
            bridge::ArtifactSourceDependency::FileContent { file, content } => Self {
                content: ArtifactSourceDependencyContent::FileContent {
                    file: FileId::from_bridge(file),
                    content: FileContentId::from_bridge(content),
                },
            },
        }
    }
}

/// One exact dependency read while building an artifact.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactDependency {
    content: ArtifactDependencyContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum ArtifactDependencyContent {
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

#[wasm_bindgen]
impl ArtifactDependency {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "artifact")]
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self {
            content: ArtifactDependencyContent::Artifact { version },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "source")]
    pub fn source(dependency: ArtifactSourceDependency) -> Self {
        Self {
            content: ArtifactDependencyContent::Source { dependency },
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            ArtifactDependencyContent::Artifact { .. } => "artifact",
            ArtifactDependencyContent::Source { .. } => "source",
        };
        label.to_string()
    }

    /// The exact artifact version depended on.
    #[wasm_bindgen(getter, js_name = "version")]
    pub fn version(&self) -> Option<ArtifactVersion> {
        match &self.content {
            ArtifactDependencyContent::Artifact { version: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The primitive source observation.
    #[wasm_bindgen(getter, js_name = "dependency")]
    pub fn dependency(&self) -> Option<ArtifactSourceDependency> {
        match &self.content {
            ArtifactDependencyContent::Source {
                dependency: value, ..
            } => Some(value.clone()),
            _ => None,
        }
    }
}

impl ArtifactDependency {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactDependency) -> Self {
        match value {
            bridge::ArtifactDependency::Artifact { version } => Self {
                content: ArtifactDependencyContent::Artifact {
                    version: ArtifactVersion::from_bridge(version),
                },
            },
            bridge::ArtifactDependency::Source { dependency } => Self {
                content: ArtifactDependencyContent::Source {
                    dependency: ArtifactSourceDependency::from_bridge(dependency),
                },
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
