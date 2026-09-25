use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{RuntimeOptions, WorldOptions};

/// Complete configuration for one World and its initial Runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOptions {
    /// World configuration.
    pub world: WorldOptions,
    /// Initial Runtime configuration.
    pub runtime: RuntimeOptions,
}

/// Execution mode for World scheduling and replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Strict execution through runtime-owned facts and effect boundaries.
    #[default]
    Strict,
    /// Fast execution without determinism guarantees.
    Fast,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

/// Replay payload selection for record/replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ReplayPayloadMode {
    /// Record only the result value.
    #[default]
    ResultsOnly,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}
