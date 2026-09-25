use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Request to read one workspace's physical revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RevisionRequest {
    /// Owning workspace root.
    pub root: PathBuf,
}

/// Request to reload one workspace root from its host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReloadRequest {
    /// Root path to reload.
    pub root: PathBuf,
}
