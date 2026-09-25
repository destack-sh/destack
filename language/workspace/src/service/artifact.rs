use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactReference;
use tspp_serde::Reflect;

use crate::ExportInput;

/// Request to read one workspace artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArtifactRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact artifact reference.
    pub artifact: ArtifactReference,
}

/// Request to materialize one artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExportRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Export operation input.
    pub input: ExportInput,
}
