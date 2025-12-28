use serde::Deserialize;

use crate::{Platform, Runtime};

/// Normalized profile configuration.
#[derive(Debug, Clone, Default)]
pub struct ProfileConfig {
    /// Runtime environment for this profile.
    pub runtime: Option<Runtime>,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform for this profile.
    pub platform: Option<Platform>,
    /// Library files for this profile.
    pub lib: Option<Vec<String>>,
    /// Debug flag exposed to import.meta.
    pub debug: Option<bool>,
    /// Comptime environment whitelist.
    pub comptime_env: Option<Vec<String>>,
}

impl ProfileConfig {
    /// Convert from a JSON profile config.
    pub fn from_json(json: &ProfileConfigJson) -> Self {
        Self {
            runtime: json.runtime.as_deref().and_then(Runtime::parse),
            runtime_version: json.runtime_version.clone(),
            platform: json.platform.as_deref().and_then(Platform::parse),
            lib: json.lib.clone(),
            debug: json.debug,
            comptime_env: json.comptime_env.clone(),
        }
    }
}

/// Profile configuration JSON (from dsconfig.json).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfileConfigJson {
    /// Runtime environment (browser, node, wasm-wasi, native-hosted, etc.).
    pub runtime: Option<String>,
    /// Runtime version for selecting versioned libs.
    pub runtime_version: Option<String>,
    /// Target platform (web, windows, macos, linux, ios, android, bare-metal, etc.).
    pub platform: Option<String>,
    /// Library files for this profile.
    pub lib: Option<Vec<String>>,
    /// Debug build flag (affects import.meta.debug).
    pub debug: Option<bool>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
}
