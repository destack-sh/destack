use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_artifact::EnvironmentKey;
use tspp_serde::Reflect;

use super::{
    DESTACK_FEATURES, DESTACK_MODES, DESTACK_PRODUCT, DESTACK_PROFILE, DESTACK_ROLES, DESTACK_TAGS,
    DESTACK_TARGET,
};

/// Virtual ambient state for one session operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    /// Working directory visible to this operation.
    pub cwd: Option<PathBuf>,
    /// Argument vector visible to this operation.
    pub args: Vec<String>,
    /// Captured environment variables visible to this operation.
    pub env: BTreeMap<String, String>,
    /// Requested condition selection.
    pub selection: ConditionSelection,
}

impl Environment {
    /// Capture ambient values from the current process.
    pub fn capture_process() -> Self {
        let cwd = env::current_dir().ok();
        let args = env::args().collect();
        let env = env::vars().collect();
        let selection = ConditionSelection::from_env(&env);

        Self {
            cwd,
            args,
            env,
            selection,
        }
    }

    /// Return one captured environment variable by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.env.get(name).map(String::as_str)
    }

    /// Return the captured environment entries in stable order.
    pub fn env_entries(&self) -> Vec<(String, String)> {
        self.env
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// Return one environment key for all captured environment variables.
    pub fn key_all(&self) -> EnvironmentKey {
        EnvironmentKey::all(self.env_entries())
    }

    /// Return one environment key for the requested environment variables.
    pub fn key_whitelist(&self, keys: &[String]) -> EnvironmentKey {
        EnvironmentKey::whitelist(keys, |key| self.get(key).map(ToOwned::to_owned))
    }
}

/// Requested source graph and profile selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionSelection {
    /// Requested build target name.
    pub target: Option<String>,
    /// Requested product name.
    pub product: Option<String>,
    /// Requested profile name.
    pub profile: Option<String>,
    /// Requested source graph modes.
    pub modes: Vec<String>,
    /// Requested source graph roles.
    pub roles: Vec<String>,
    /// Requested source graph features.
    pub features: Vec<String>,
    /// Requested source graph tags.
    pub tags: Vec<String>,
}

impl ConditionSelection {
    /// Build condition selection from captured environment variables.
    pub fn from_env(env: &BTreeMap<String, String>) -> Self {
        Self {
            target: env.get(DESTACK_TARGET).cloned(),
            product: env.get(DESTACK_PRODUCT).cloned(),
            profile: env.get(DESTACK_PROFILE).cloned(),
            modes: condition_list(env.get(DESTACK_MODES)),
            roles: condition_list(env.get(DESTACK_ROLES)),
            features: condition_list(env.get(DESTACK_FEATURES)),
            tags: condition_list(env.get(DESTACK_TAGS)),
        }
    }
}

/// Split one comma separated condition selector list.
fn condition_list(value: Option<&String>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
