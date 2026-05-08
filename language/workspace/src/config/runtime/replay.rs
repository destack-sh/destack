use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{ReplayPayloadMode, ReplayPayloadModeJson};

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
/// Replay options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ReplayOptionsJson {
    /// Base path for replay logs (file or directory).
    pub path: Option<String>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Replay payload selection for record mode.
    pub payload: Option<ReplayPayloadModeJson>,
}

impl ReplayOptionsJson {
    /// Apply replay overrides to a base set of options.
    pub fn apply_to(&self, options: &mut ReplayOptions) {
        // apply path overrides
        if let Some(path) = &self.path {
            options.path = Some(PathBuf::from(path));
        }

        // apply template overrides
        if let Some(template) = &self.template {
            options.template = Some(template.clone());
        }

        // apply chunk sizing overrides
        if let Some(chunk_size_mb) = self.chunk_size_mb {
            options.chunk_size_mb = Some(chunk_size_mb);
        }

        // apply replay payload overrides
        if let Some(payload) = self.payload {
            options.payload = payload.into();
        }
    }
}
