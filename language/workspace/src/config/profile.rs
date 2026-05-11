use std::path::PathBuf;

use destack_artifact::{Host, Platform};
use serde::Deserialize;

use crate::config::{RuntimeConfigJson, RuntimeOptions, runtime_options_from_json};

/// Normalized profile options.
#[derive(Debug, Clone, Default)]
pub struct ProfileOptions {
    /// Runtime contract for this profile.
    pub runtime: Option<RuntimeOptions>,
    /// Target platform / operating system for this profile.
    pub platform: Option<Platform>,
    /// Target host environment for this profile.
    pub host: Option<Host>,
    /// Comptime environment whitelist.
    pub comptime_env: Option<Vec<String>>,
    /// Active source graph modes for this profile.
    pub modes: Vec<String>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules for this profile.
    pub globals: Vec<PathBuf>,
    /// Derive providers automatically considered in this profile.
    pub derive: Vec<String>,
}

impl ProfileOptions {
    /// Convert from JSON profile options.
    pub fn from_json(json: &ProfileOptionsJson) -> Result<Self, String> {
        let platform = json
            .platform
            .as_deref()
            .map(parse_profile_platform)
            .transpose()?;
        let host = json.host.as_deref().map(parse_profile_host).transpose()?;

        Ok(Self {
            runtime: json
                .runtime
                .as_ref()
                .map(|runtime| runtime_options_from_json(Some(runtime))),
            platform,
            host,
            comptime_env: json.comptime_env.clone(),
            modes: json.modes.clone().unwrap_or_default(),
            tree: json.tree.clone(),
            globals: json
                .globals
                .as_ref()
                .map(|globals| globals.iter().map(PathBuf::from).collect())
                .unwrap_or_default(),
            derive: json.derive.clone().unwrap_or_default(),
        })
    }
}

/// Parse one profile platform from a config value.
fn parse_profile_platform(value: &str) -> Result<Platform, String> {
    Platform::parse(value).ok_or_else(|| format!("unsupported profile platform '{value}'"))
}

/// Parse one profile host from a config value.
fn parse_profile_host(value: &str) -> Result<Host, String> {
    Host::parse(value).ok_or_else(|| format!("unsupported profile host '{value}'"))
}

/// Profile options JSON from `destack.json`.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProfileOptionsJson {
    /// Runtime contract for this profile.
    pub runtime: Option<RuntimeConfigJson>,
    /// Target platform / operating system.
    pub platform: Option<String>,
    /// Target host environment.
    pub host: Option<String>,
    /// Comptime environment whitelist (if omitted, all env keys are visible).
    pub comptime_env: Option<Vec<String>>,
    /// Active source graph modes for this profile.
    pub modes: Option<Vec<String>>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules for this profile.
    pub globals: Option<Vec<String>>,
    /// Derive providers automatically considered in this profile.
    pub derive: Option<Vec<String>>,
}
