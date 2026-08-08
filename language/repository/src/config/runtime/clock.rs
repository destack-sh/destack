use serde::{Deserialize, Serialize};

/// Runtime clock configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ClockOptions {
    /// Runtime wall-clock epoch in nanoseconds.
    pub epoch_ns: Option<u64>,
}
