use destack_artifact::{HostEnvironmentKey, ProfileFlags};
use serde::{Deserialize, Serialize};

use crate::HostEnvironment;
use crate::config::{CompilerOptions, Mode};

/// The stable profile id.
pub use destack_source::ProfileId;

/// Resolved environment values for one profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEnvironment {
    /// The exposed environment entries.
    pub values: Vec<(String, String)>,
    /// The effective node environment mode.
    pub node_env: Option<String>,
    /// True when the debug mode is active.
    pub debug: bool,
    /// True when the development mode is active.
    pub dev: bool,
    /// True when the production mode is active.
    pub prod: bool,
    /// True when the test mode is active.
    pub test: bool,
    /// True when the benchmark mode is active.
    pub bench: bool,
    /// True when the lint mode is active.
    pub lint: bool,
    /// Active source graph modes.
    pub modes: Vec<String>,
}

impl ProfileEnvironment {
    /// Derive the effective node environment.
    pub fn node_env_from_key(
        key: &HostEnvironmentKey,
        host: &HostEnvironment,
        modes: &[String],
    ) -> Option<String> {
        let has_node_env = key.keys().iter().any(|key| key == "NODE_ENV");
        if has_node_env && let Some(node_env) = host.get("NODE_ENV") {
            return Some(node_env.to_string());
        }

        if has_mode(modes, Mode::TEST) {
            Some("test".to_string())
        } else if has_mode(modes, Mode::PROD) {
            Some("production".to_string())
        } else if has_mode(modes, Mode::DEV) {
            Some("development".to_string())
        } else {
            None
        }
    }

    /// Build one profile environment from one host environment key.
    pub fn from_key(key: &HostEnvironmentKey, host: &HostEnvironment, modes: &[String]) -> Self {
        let node_env = Self::node_env_from_key(key, host, modes);
        let debug = has_mode(modes, Mode::DEBUG);
        let dev = has_mode(modes, Mode::DEV);
        let prod = has_mode(modes, Mode::PROD);
        let test = has_mode(modes, Mode::TEST);
        let bench = has_mode(modes, Mode::BENCH);
        let lint = has_mode(modes, Mode::LINT);
        let modes = modes.to_vec();
        let mut values = key
            .keys()
            .iter()
            .filter_map(|key| host.get(key).map(|value| (key.clone(), value.to_string())))
            .collect::<Vec<_>>();

        let has_node_env = key.keys().iter().any(|key| key == "NODE_ENV");

        if has_node_env
            && let Some(node_env_value) = node_env.clone()
            && !values.iter().any(|(key, _)| key == "NODE_ENV")
        {
            values.push(("NODE_ENV".to_string(), node_env_value));
        }

        values.sort_by(|left, right| left.0.cmp(&right.0));

        Self {
            values,
            node_env,
            debug,
            dev,
            prod,
            test,
            bench,
            lint,
            modes,
        }
    }
}

/// Return whether one active mode is present.
fn has_mode(modes: &[String], mode: Mode) -> bool {
    modes.iter().any(|active| active == mode.name)
}

/// Derive profile identity flags from compiler options.
pub fn profile_flags_for_compiler_options(options: &CompilerOptions) -> ProfileFlags {
    ProfileFlags {
        no_managed: !options.no_managed.is_allow(),
        no_heap: !options.no_heap.is_allow(),
        no_runtime: !options.no_runtime.is_allow(),
        no_internal_import: !options.no_internal_import.is_allow(),
        no_implicit_dynamic_dispatch: !options.no_implicit_dynamic_dispatch.is_allow(),
    }
}
