use serde::{Deserialize, Serialize};

use super::WorkspaceHandleId;

/// Artifact fetch request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactRequest {
    /// Fetch artifact content by id.
    Fetch {
        handle: WorkspaceHandleId,
        artifact_id: u64,
    },
}

/// Artifact fetch response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactResponse {
    /// Artifact fetch accepted.
    FetchAccepted {
        handle: WorkspaceHandleId,
        artifact_id: u64,
    },
}
