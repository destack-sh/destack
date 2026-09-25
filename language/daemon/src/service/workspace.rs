use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Request to open one daemon workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct OpenWorkspaceRequest {
    /// Requested workspace root.
    pub root: PathBuf,
}

/// One opened daemon workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct OpenWorkspaceResponse {
    /// Canonical workspace root.
    pub root: PathBuf,
}

/// Request to release one daemon workspace from the calling connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CloseWorkspaceRequest {
    /// Canonical workspace root returned when it was opened.
    pub root: PathBuf,
}
