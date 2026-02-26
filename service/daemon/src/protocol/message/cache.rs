use serde::{Deserialize, Serialize};

use destack_source::TargetId;

use super::WorkspaceHandleId;

/// Cache control request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheRequest {
    /// Clear all cache entries for a workspace.
    Clear { handle: WorkspaceHandleId },
    /// Evict cache entries for a workspace.
    Evict {
        handle: WorkspaceHandleId,
        targets: Vec<TargetId>,
    },
    /// Warm cache entries for a workspace.
    Warm {
        handle: WorkspaceHandleId,
        targets: Vec<TargetId>,
    },
}

/// Cache control response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the operation succeeded.
    pub success: bool,
}
