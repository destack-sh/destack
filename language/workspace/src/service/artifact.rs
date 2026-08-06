use std::path::PathBuf;

use destack_artifact::ArtifactReference;
use destack_serde::Reflect;
use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use crate::ExportInput;

/// Request to read one workspace artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArtifactRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact artifact reference.
    pub artifact: ArtifactReference,
}

/// Request to store one content value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StoreRequest {
    /// Content value to store.
    pub content: Content,
}

/// Request to load one content value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct LoadRequest {
    /// Content identifier to load.
    pub content: ContentId,
}

/// Request to materialize one artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExportRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Export operation input.
    pub input: ExportInput,
}
