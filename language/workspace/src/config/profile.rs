use std::path::PathBuf;

use destack_artifact::{Host, Platform};
use serde::{Deserialize, Serialize};

use crate::config::{CompilerRestrictions, Derive, RuntimeOptions, Stage};

/// Normalized profile options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileOptions {
    /// Release stage for this profile.
    pub stage: Option<Stage>,
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
    /// Active source graph roles for this profile.
    pub roles: Vec<String>,
    /// Active source graph features for this profile.
    pub features: Vec<String>,
    /// Active source graph tags for this profile.
    pub tags: Vec<String>,
    /// Default tree tag builder provider.
    pub tree: Option<String>,
    /// Global provider modules for this profile.
    pub globals: Vec<PathBuf>,
    /// Well-known derives automatically considered in this profile.
    pub derive: Vec<Derive>,
    /// Static semantic restrictions for this profile.
    pub restrictions: CompilerRestrictions,
}
