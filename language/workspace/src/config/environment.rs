use serde::Deserialize;

use indexmap::IndexMap;

use crate::config::runtime::{RuntimeConfigJson, RuntimeOptionsJson};

/// Environment options.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentOptions {
    /// Default profile selection for this environment.
    pub profile: Option<String>,
    /// Default active source graph modes for this environment.
    pub modes: Vec<String>,
    /// Runtime overrides for this environment.
    pub runtime: Option<RuntimeOptionsJson>,
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

        if let Some(parent_runtime) = &parent.runtime {
            if let Some(runtime) = self.runtime.as_mut() {
                runtime.extend_from(parent_runtime);
            } else {
                self.runtime = Some(parent_runtime.clone());
            }
        }
    }
}

impl From<&EnvironmentJson> for EnvironmentOptions {
    fn from(json: &EnvironmentJson) -> Self {
        Self {
            profile: json.profile.clone(),
            modes: json.modes.clone().unwrap_or_default(),
            runtime: json
                .runtime
                .as_ref()
                .map(RuntimeConfigJson::as_options_json),
        }
    }
}

/// Convert environment declarations into normalized options.
pub fn environment_options_from_json(
    json: &Option<IndexMap<String, EnvironmentJson>>,
) -> IndexMap<String, EnvironmentOptions> {
    json.as_ref()
        .map(|environments| {
            environments
                .iter()
                .map(|(name, environment)| (name.clone(), EnvironmentOptions::from(environment)))
                .collect()
        })
        .unwrap_or_default()
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

/// A reusable toolchain and runtime environment.
///
/// Inputs: toolchain selections, runtime overrides, and reusable policy references.
/// Outputs: one named context for checking, building, and running the package.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentJson {
    /// Default profile selection for this environment.
    pub profile: Option<String>,
    /// Default active source graph modes for this environment.
    pub modes: Option<Vec<String>>,
    /// Runtime overrides for this environment.
    pub runtime: Option<RuntimeConfigJson>,
}
