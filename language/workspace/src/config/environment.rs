use serde::{Deserialize, Serialize};

use indexmap::IndexMap;

use crate::config::runtime::RuntimeOptions;

/// Environment options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentOptions {
    /// Default profile selection for this environment.
    pub profile: Option<String>,
    /// Default active source graph modes for this environment.
    pub modes: Vec<String>,
    /// Runtime overrides for this environment.
    pub runtime: Option<RuntimeOptions>,
}

impl EnvironmentOptions {
    /// Inherit unset environment settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.profile.is_none() {
            self.profile = parent.profile.clone();
        }
        if self.modes.is_empty() {
            self.modes = parent.modes.clone();
        }

        if self.runtime.is_none() {
            self.runtime = parent.runtime.clone();
        }
    }
}

/// Inherit one environment map from a parent config.
pub fn extend_environment_options(
    current: &mut IndexMap<String, EnvironmentOptions>,
    parent: &IndexMap<String, EnvironmentOptions>,
) {
    for (name, environment) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(environment);
        } else {
            current.insert(name.clone(), environment.clone());
        }
    }
}
