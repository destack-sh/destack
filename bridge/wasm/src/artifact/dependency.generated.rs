// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ArtifactVersion, ContentId, FileId};

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
        /// The source file id.
        file: FileId,
        /// The exact source content id.
        content: ContentId,
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
    pub fn source(file: FileId, content: ContentId) -> Self {
        Self {
            content: ArtifactDependencyContent::Source { file, content },
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
    #[wasm_bindgen(js_name = "getVersion")]
    pub fn get_version(&self) -> Option<ArtifactVersion> {
        match &self.content {
            ArtifactDependencyContent::Artifact { version: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The source file id.
    #[wasm_bindgen(js_name = "getFile")]
    pub fn get_file(&self) -> Option<FileId> {
        match &self.content {
            ArtifactDependencyContent::Source { file: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// The exact source content id.
    #[wasm_bindgen(js_name = "getContent")]
    pub fn get_content(&self) -> Option<ContentId> {
        match &self.content {
            ArtifactDependencyContent::Source { content: value, .. } => Some(value.clone()),
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
            bridge::ArtifactDependency::Source { file, content } => Self {
                content: ArtifactDependencyContent::Source {
                    file: FileId::from_bridge(file),
                    content: ContentId::from_bridge(content),
                },
            },
        }
    }
}
