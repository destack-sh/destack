use serde::{Deserialize, Serialize};

/// Time source selection for runtime clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TimeMode {
    /// Use the host clock directly.
    #[default]
    Host,
    /// Use a virtualized clock derived from runtime state.
    Virtual,
}
/// Runtime clock configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TimeOptions {
    /// Clock mode selection.
    pub mode: TimeMode,
    /// Epoch in nanoseconds for virtual time.
    pub epoch_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}
/// Runtime time options for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TimeOptionsJson {
    /// Clock mode selection.
    pub mode: Option<TimeModeJson>,
    /// Epoch in nanoseconds for virtual time.
    pub epoch_ns: Option<u64>,
    /// Time zone identifier or fixed offset string.
    pub time_zone: Option<String>,
}

impl TimeOptionsJson {
    /// Inherit unset time settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }

        if self.epoch_ns.is_none() {
            self.epoch_ns = parent.epoch_ns;
        }

        if self.time_zone.is_none() {
            self.time_zone = parent.time_zone.clone();
        }
    }

    /// Apply time overrides to a base set of options.
    pub fn apply_to(&self, options: &mut TimeOptions) {
        // apply mode overrides
        if let Some(mode) = self.mode {
            options.mode = TimeMode::from(mode);
        }

        // apply epoch overrides
        if let Some(epoch_ns) = self.epoch_ns {
            options.epoch_ns = Some(epoch_ns);
        }

        // apply timezone overrides
        if let Some(time_zone) = &self.time_zone {
            options.time_zone = Some(time_zone.clone());
        }
    }
}
/// Time mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TimeModeJson {
    /// Use the host clock directly.
    Host,
    /// Use a virtualized clock derived from runtime state.
    Virtual,
}

impl From<TimeModeJson> for TimeMode {
    fn from(value: TimeModeJson) -> Self {
        match value {
            TimeModeJson::Host => TimeMode::Host,
            TimeModeJson::Virtual => TimeMode::Virtual,
        }
    }
}
