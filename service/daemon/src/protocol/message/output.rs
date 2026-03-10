use serde::{Deserialize, Serialize};

use super::WorkspaceHandleId;

/// Output fetch request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputRequest {
    /// Fetch output content by id.
    Fetch {
        handle: WorkspaceHandleId,
        output_id: u64,
    },
}

/// Output fetch response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputResponse {
    /// Output fetch accepted.
    FetchAccepted {
        handle: WorkspaceHandleId,
        output_id: u64,
    },
}
