use serde::{Deserialize, Serialize};

/// Randomness source selection for the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RandomMode {
    /// Use the host randomness source.
    #[default]
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}
/// Runtime randomness configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RandomOptions {
    /// Randomness source selection.
    pub mode: RandomMode,
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use a per-runnable random stream.
    pub per_runnable: bool,
}
/// Runtime randomness options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RandomOptionsJson {
    /// Randomness source selection.
    pub mode: Option<RandomModeJson>,
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use a per-runnable random stream.
    pub per_runnable: Option<bool>,
}

impl RandomOptionsJson {
    /// Inherit unset random settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }

        if self.seed.is_none() {
            self.seed = parent.seed;
        }

        if self.per_runnable.is_none() {
            self.per_runnable = parent.per_runnable;
        }
    }

    /// Apply random overrides to a base set of options.
    pub fn apply_to(&self, options: &mut RandomOptions) {
        // apply mode overrides
        if let Some(mode) = self.mode {
            options.mode = RandomMode::from(mode);
        }

        // apply seed overrides
        if let Some(seed) = self.seed {
            options.seed = Some(seed);
        }

        // apply per-runnable overrides
        if let Some(per_runnable) = self.per_runnable {
            options.per_runnable = per_runnable;
        }
    }
}
/// Randomness mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RandomModeJson {
    /// Use the host randomness source.
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

impl From<RandomModeJson> for RandomMode {
    fn from(value: RandomModeJson) -> Self {
        match value {
            RandomModeJson::Host => RandomMode::Host,
            RandomModeJson::Deterministic => RandomMode::Deterministic,
        }
    }
}
