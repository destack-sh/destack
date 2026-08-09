use std::path::PathBuf;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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
