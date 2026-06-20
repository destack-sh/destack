// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactVersion, ContentId, FileId};

/// One exact dependency read while building an artifact.
#[derive(Debug)]
#[napi(object, js_name = "ArtifactDependency")]
pub struct ArtifactDependency {
    /// Payload variant label.
    pub kind: String,
    /// The exact artifact version depended on.
    pub version: Option<ArtifactVersion>,
    /// The source file id.
    pub file: Option<FileId>,
    /// The exact source content id.
    pub content: Option<ContentId>,
}

impl ArtifactDependency {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactDependency) -> Self {
        match value {
            bridge::ArtifactDependency::Artifact { version } => Self {
                kind: "artifact".to_string(),
                version: Some(ArtifactVersion::from_bridge(version)),
                file: None,
                content: None,
            },
            bridge::ArtifactDependency::Source { file, content } => Self {
                kind: "source".to_string(),
                file: Some(FileId::from_bridge(file)),
                content: Some(ContentId::from_bridge(content)),
                version: None,
            },
        }
    }
}
