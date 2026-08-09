use std::path::PathBuf;

use destack_repository::Revision;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::ReloadReason;

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

/// Request to reload one workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReloadRootRequest {
    /// Root path to reload.
    pub root: PathBuf,
    /// Reload reason.
    pub reason: ReloadReason,
}

/// Request to read one workspace root revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadRevisionRequest {
    /// Root path to read.
    pub root: PathBuf,
}
