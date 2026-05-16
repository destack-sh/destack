use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ReplayPayloadMode;

/// Replay configuration for runtime record/replay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayOptions {
    /// Base path for replay logs (file or directory).
    pub path: Option<PathBuf>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Replay payload selection for record mode.
    pub payload: ReplayPayloadMode,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            path: None,
            template: None,
            chunk_size_mb: None,
            payload: ReplayPayloadMode::ResultsOnly,
        }
    }
}
