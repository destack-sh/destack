use std::collections::BTreeMap;
use std::path::PathBuf;

use destack_artifact::EnvironmentKey;
use serde::{Deserialize, Serialize};

/// Environment key used to override the cache directory.
pub const DESTACK_CACHE_DIR: &str = "DESTACK_CACHE_DIR";
/// Environment key used to override the watch mode.
pub const DESTACK_WATCH_MODE: &str = "DESTACK_WATCH_MODE";
/// Environment key used to override the watch poll interval in milliseconds.
pub const DESTACK_WATCH_POLL_MS: &str = "DESTACK_WATCH_POLL_MS";
/// Environment key used to override the watch debounce interval in milliseconds.
pub const DESTACK_WATCH_DEBOUNCE_MS: &str = "DESTACK_WATCH_DEBOUNCE_MS";
/// Environment key used to override the worker count.
pub const DESTACK_WORKERS: &str = "DESTACK_WORKERS";
/// Environment key used to override the default target selection.
pub const DESTACK_TARGET: &str = "DESTACK_TARGET";
/// Environment key used to override the default product selection.
pub const DESTACK_PRODUCT: &str = "DESTACK_PRODUCT";
/// Environment key used to override the default profile selection.
pub const DESTACK_PROFILE: &str = "DESTACK_PROFILE";
/// Environment key used to override the default active modes.
pub const DESTACK_MODES: &str = "DESTACK_MODES";
/// Environment key used to override the default active roles.
pub const DESTACK_ROLES: &str = "DESTACK_ROLES";
/// Environment key used to override the default active features.
pub const DESTACK_FEATURES: &str = "DESTACK_FEATURES";
/// Environment key used to override the default active tags.
pub const DESTACK_TAGS: &str = "DESTACK_TAGS";
/// Environment key used to override the output directory for targets.
pub const DESTACK_OUT_DIR: &str = "DESTACK_OUT_DIR";
/// Environment key used to override the output file for single file targets.
pub const DESTACK_OUT_FILE: &str = "DESTACK_OUT_FILE";
/// Environment key used to override the declaration output directory.
pub const DESTACK_DECLARATION_DIR: &str = "DESTACK_DECLARATION_DIR";
/// Environment key used to override compiler logging filters.
pub const DESTACK_LOG: &str = "DESTACK_LOG";
/// Environment key for xdg cache home.
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
/// Environment key for home directory on unix.
pub const HOME: &str = "HOME";
/// Environment key for local app data on windows.
pub const LOCAL_APPDATA: &str = "LOCALAPPDATA";
/// Environment key for user profile on windows.
pub const USERPROFILE: &str = "USERPROFILE";

/// Requested source graph and profile selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Virtual ambient state for one session operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        let cwd = std::env::current_dir().ok();
        let args = std::env::args().collect();
        let env = std::env::vars().collect();
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
