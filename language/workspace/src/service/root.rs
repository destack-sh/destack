use std::path::PathBuf;

use destack_repository::Revision;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Request to open one workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct OpenRootRequest {
    /// Requested root path.
    pub root: PathBuf,
}

/// One opened workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct OpenRootResponse {
    /// Canonical root path.
    pub root: PathBuf,
    /// Current semantic revision.
    pub revision: Revision,
}

/// Request to reload one workspace root from its host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReloadRequest {
    /// Root path to reload.
    pub root: PathBuf,
}

/// Request to read one workspace root revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadRevisionRequest {
    /// Root path to read.
    pub root: PathBuf,
}
