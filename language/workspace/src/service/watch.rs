use std::path::PathBuf;
use std::time::Duration;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::WatchPolicy;

/// Request to watch workspace roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct WatchRequest {
    /// Roots to watch.
    pub roots: Vec<PathBuf>,
    /// Maximum event coalescing window in milliseconds.
    pub coalesce_window_milliseconds: u64,
    /// Maximum event and status count per batch.
    pub max_batch_size: usize,
}

impl WatchRequest {
    /// Return this request's workspace watch policy.
    pub fn policy(&self) -> WatchPolicy {
        WatchPolicy {
            coalesce_window: Duration::from_millis(self.coalesce_window_milliseconds),
            max_batch_size: self.max_batch_size,
        }
    }
}
