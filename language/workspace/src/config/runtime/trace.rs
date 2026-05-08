use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{ReplayPayloadMode, ReplayPayloadModeJson};

/// Trace recording mode for one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
pub enum RestoreMode {
    /// Restore one live logical world state.
    #[default]
    World,
    /// Restore one exact serialized execution image.
    Image,
}

/// Runtime trace configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

/// Runtime trace configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TraceOptionsJson {
    /// The configured trace mode.
    pub mode: Option<TraceModeJson>,
    /// The configured restore contract.
    pub restore: Option<RestoreModeJson>,
    /// Base path for trace logs.
    pub path: Option<String>,
    /// Template for auto-generated log file names.
    pub template: Option<String>,
    /// Chunk size in megabytes for log rotation.
    pub chunk_size_mb: Option<u64>,
    /// Trace payload selection for recorded binding calls.
    pub payload: Option<ReplayPayloadModeJson>,
}

impl TraceOptionsJson {
    /// Inherit unset trace settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }

        if self.restore.is_none() {
            self.restore = parent.restore;
        }

        if self.path.is_none() {
            self.path = parent.path.clone();
        }

        if self.template.is_none() {
            self.template = parent.template.clone();
        }

        if self.chunk_size_mb.is_none() {
            self.chunk_size_mb = parent.chunk_size_mb;
        }

        if self.payload.is_none() {
            self.payload = parent.payload;
        }
    }

    /// Apply trace overrides to one base set of options.
    pub fn apply_to(&self, options: &mut TraceOptions) {
        // mode
        if let Some(mode) = self.mode {
            options.mode = mode.into();
        }

        // restore contract
        if let Some(restore) = self.restore {
            options.restore = restore.into();
        }

        // storage
        if let Some(path) = &self.path {
            options.path = Some(PathBuf::from(path));
        }

        if let Some(template) = &self.template {
            options.template = Some(template.clone());
        }

        if let Some(chunk_size_mb) = self.chunk_size_mb {
            options.chunk_size_mb = Some(chunk_size_mb);
        }

        // payload
        if let Some(payload) = self.payload {
            options.payload = payload.into();
        }
    }
}

/// Trace mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TraceModeJson {
    /// Do not record or replay one trace log.
    Off,
    /// Record one trace log from live execution.
    Record,
    /// Replay one trace log as authoritative input.
    Replay,
}

impl From<TraceModeJson> for TraceMode {
    fn from(value: TraceModeJson) -> Self {
        match value {
            TraceModeJson::Off => TraceMode::Off,
            TraceModeJson::Record => TraceMode::Record,
            TraceModeJson::Replay => TraceMode::Replay,
        }
    }
}

/// Restore contract for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RestoreModeJson {
    /// Restore one live logical world state.
    World,
    /// Restore one exact serialized execution image.
    Image,
}

impl From<RestoreModeJson> for RestoreMode {
    fn from(value: RestoreModeJson) -> Self {
        match value {
            RestoreModeJson::World => RestoreMode::World,
            RestoreModeJson::Image => RestoreMode::Image,
        }
    }
}
