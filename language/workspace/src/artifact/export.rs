use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactReference;
use tspp_core::Blob;
use tspp_serde::Reflect;

/// Request to materialize derived outputs on the workspace host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExportInput {
    /// Artifact to export.
    pub artifact: ArtifactReference,
    /// Output directory on the workspace host.
    pub directory: PathBuf,
    /// Whether existing files may be overwritten.
    pub overwrite: bool,
}

/// Result of materializing derived outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExportResult {
    /// Files written on the workspace host.
    pub files: Vec<ExportedFile>,
}

/// One file written by export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ExportedFile {
    /// Path written on the workspace host.
    pub path: PathBuf,
    /// Blob written to the path.
    pub blob: Blob,
}
