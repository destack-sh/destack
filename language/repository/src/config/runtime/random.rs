use serde::{Deserialize, Serialize};

/// Runtime randomness configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RandomOptions {
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
}
