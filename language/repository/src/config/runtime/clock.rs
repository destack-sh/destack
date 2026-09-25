use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// World clock configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ClockOptions {
    /// World wall-clock epoch in nanoseconds.
    pub epoch_nanos: Option<u64>,
}
