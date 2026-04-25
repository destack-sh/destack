use serde::{Deserialize, Serialize};

/// Simulation settings for time facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TimeSimulationOptions {
    /// Epoch in nanoseconds for simulated wall time.
    pub epoch_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}

/// Simulation settings for randomness facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RandomSimulationOptions {
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use one random stream per runnable.
    pub per_runnable: bool,
}

/// Runtime simulation configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SimulationOptions {
    /// Simulation settings for time facts.
    pub time: TimeSimulationOptions,
    /// Simulation settings for randomness facts.
    pub random: RandomSimulationOptions,
}

/// Simulation settings for time facts for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TimeSimulationOptionsJson {
    /// Epoch in nanoseconds for simulated wall time.
    pub epoch_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}

impl TimeSimulationOptionsJson {
    /// Inherit unset simulation settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.epoch_ns.is_none() {
            self.epoch_ns = parent.epoch_ns;
        }

        if self.time_zone.is_none() {
            self.time_zone = parent.time_zone.clone();
        }
    }

    /// Apply simulation overrides to one base set of options.
    pub fn apply_to(&self, options: &mut TimeSimulationOptions) {
        if let Some(epoch_ns) = self.epoch_ns {
            options.epoch_ns = Some(epoch_ns);
        }

        if let Some(time_zone) = &self.time_zone {
            options.time_zone = Some(time_zone.clone());
        }
    }
}

/// Simulation settings for randomness facts for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RandomSimulationOptionsJson {
    /// Seed for deterministic randomness streams.
    pub seed: Option<u64>,
    /// Whether to use one random stream per runnable.
    pub per_runnable: Option<bool>,
}

impl RandomSimulationOptionsJson {
    /// Inherit unset simulation settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.seed.is_none() {
            self.seed = parent.seed;
        }

        if self.per_runnable.is_none() {
            self.per_runnable = parent.per_runnable;
        }
    }

    /// Apply simulation overrides to one base set of options.
    pub fn apply_to(&self, options: &mut RandomSimulationOptions) {
        if let Some(seed) = self.seed {
            options.seed = Some(seed);
        }

        if let Some(per_runnable) = self.per_runnable {
            options.per_runnable = per_runnable;
        }
    }
}

/// Runtime simulation configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SimulationOptionsJson {
    /// Simulation settings for time facts.
    pub time: Option<TimeSimulationOptionsJson>,
    /// Simulation settings for randomness facts.
    pub random: Option<RandomSimulationOptionsJson>,
}

impl SimulationOptionsJson {
    /// Inherit unset simulation settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(time) = &mut self.time {
            if let Some(parent_time) = &parent.time {
                time.extend_from(parent_time);
            }
        } else {
            self.time = parent.time.clone();
        }

        if let Some(random) = &mut self.random {
            if let Some(parent_random) = &parent.random {
                random.extend_from(parent_random);
            }
        } else {
            self.random = parent.random.clone();
        }
    }

    /// Apply simulation overrides to one base set of options.
    pub fn apply_to(&self, options: &mut SimulationOptions) {
        if let Some(time) = &self.time {
            time.apply_to(&mut options.time);
        }

        if let Some(random) = &self.random {
            random.apply_to(&mut options.random);
        }
    }
}
