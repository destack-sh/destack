use serde::{Deserialize, Serialize};

/// Randomness source selection for the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RandomSource {
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
    pub source: RandomSource,
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
    pub source: Option<RandomSourceJson>,
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use a per-runnable random stream.
    pub per_runnable: Option<bool>,
}

impl RandomOptionsJson {
    /// Inherit unset random settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.source.is_none() {
            self.source = parent.source;
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
        // apply source overrides
        if let Some(source) = self.source {
            options.source = RandomSource::from(source);
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
/// Randomness source for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RandomSourceJson {
    /// Use the host randomness source.
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

impl From<RandomSourceJson> for RandomSource {
    fn from(value: RandomSourceJson) -> Self {
        match value {
            RandomSourceJson::Host => RandomSource::Host,
            RandomSourceJson::Deterministic => RandomSource::Deterministic,
        }
    }
}
