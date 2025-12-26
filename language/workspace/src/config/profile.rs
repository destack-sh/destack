use serde::Deserialize;

use crate::{OutputFormat, Platform, Runtime};

use super::dsconfig::OutputFormatJson;

/// Normalized profile configuration.
#[derive(Debug, Clone, Default)]
pub struct ProfileConfig {
    /// Output format for this profile.
    pub output: Option<OutputFormat>,
    /// Runtime environment for this profile.
    pub runtime: Option<Runtime>,
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
            output: json.output.map(OutputFormat::from),
            runtime: json.runtime.as_deref().and_then(Runtime::parse),
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
    /// Output format (js, ts, wasm, native).
    pub output: Option<OutputFormatJson>,
    /// Runtime environment (browser, node, wasm-wasi, destack, etc.).
    pub runtime: Option<String>,
    /// Target platform (web, windows, macos, linux, ios, android, etc.).
    pub platform: Option<String>,
    /// Library files for this profile.
    pub lib: Option<Vec<String>>,
    /// Debug build flag (affects import.meta.debug).
    pub debug: Option<bool>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
}
