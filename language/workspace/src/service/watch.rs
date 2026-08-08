use std::path::PathBuf;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Request to watch one workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WatchRequest {
    /// Root to watch.
    pub root: PathBuf,
}
