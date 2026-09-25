use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{ClockOptions, ExecutionMode, RandomOptions, ReplayPayloadMode};

/// World configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct WorldOptions {
    /// Execution mode for scheduling and effect handling.
    pub mode: ExecutionMode,
    /// World clock seed configuration.
    pub clock: ClockOptions,
    /// World randomness source configuration.
    pub random: RandomOptions,
    /// Recorded binding call payload selection.
    pub replay_payload: ReplayPayloadMode,
}
