use destack_artifact::Platform;
use serde::Deserialize;

use crate::config::{RuntimeConfigJson, RuntimeOptions, runtime_options_from_json};

/// Normalized profile options.
#[derive(Debug, Clone, Default)]
pub struct ProfileOptions {
    /// Runtime environment for this profile.
    pub runtime: Option<RuntimeOptions>,
    /// Target platform / operating system for this profile.
    pub platform: Option<Platform>,
    /// Library files for this profile.
    pub lib: Option<Vec<String>>,
    /// Additional library types for this profile.
    pub types: Option<Vec<String>>,
    /// Debug flag exposed to import.meta.
    pub debug: Option<bool>,
    /// Comptime environment whitelist.
    pub comptime_env: Option<Vec<String>>,
}

impl ProfileOptions {
    /// Convert from JSON profile options.
    pub fn from_json(json: &ProfileOptionsJson) -> Self {
        Self {
            runtime: json
                .runtime
                .as_ref()
                .map(|runtime| runtime_options_from_json(Some(runtime))),
            platform: json.platform.as_deref().and_then(Platform::parse),
            lib: json.lib.clone(),
            types: json.types.clone(),
            debug: json.debug,
            comptime_env: json.comptime_env.clone(),
        }
    }
}

/// Profile options JSON from `destack.json`.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfileOptionsJson {
    /// Runtime environment (browser, node, wasm-wasi, native-managed, etc.).
    pub runtime: Option<RuntimeConfigJson>,
    /// Target platform / operating system.
    pub platform: Option<String>,
    /// Library files for this profile.
    pub lib: Option<Vec<String>>,
    /// Additional library types for this profile.
    pub types: Option<Vec<String>>,
    /// Debug build flag (affects import.meta.debug).
    pub debug: Option<bool>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
}
