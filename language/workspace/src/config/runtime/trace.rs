use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ReplayPayloadMode;

/// Trace recording mode for one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TraceMode {
    /// Do not record or replay one trace log.
    #[default]
    Off,
    /// Record one trace log from live execution.
    Record,
    /// Replay one trace log as authoritative input.
    Replay,
}

/// Restore contract for one world checkpoint or replay boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RestoreMode {
    /// Restore one live logical world state.
    #[default]
    World,
    /// Restore one exact serialized execution image.
    Image,
}

/// Runtime trace configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TraceOptions {
    /// The configured trace mode.
    pub mode: TraceMode,
    /// The configured restore contract.
    pub restore: RestoreMode,
    /// Base path for trace logs.
    pub path: Option<PathBuf>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Trace payload selection for recorded binding calls.
    pub payload: ReplayPayloadMode,
}
