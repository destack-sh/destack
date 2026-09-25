use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Request to watch one workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WatchRequest {
    /// Root to watch.
    pub root: PathBuf,
}

/// Request to watch one workspace branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WatchBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch to watch.
    pub name: String,
}
