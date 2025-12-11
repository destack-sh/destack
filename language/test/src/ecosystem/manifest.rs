//! Package manifest parsing for ecosystem tests.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Tier of ecosystem testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Parse = 1,
    Check = 2,
    CompileJs = 3,
    CompileNative = 4,
}

impl Tier {
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Parse => "parse",
            Tier::Check => "check",
            Tier::CompileJs => "compile_js",
            Tier::CompileNative => "compile_native",
        }
    }
}

/// Package manifest (parsed from TOML).
#[derive(Debug, Clone, Deserialize)]
pub struct EcosystemManifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub tiers: TierConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiscoveryConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TierConfig {
    #[serde(default)]
    pub parse: bool,
    #[serde(default)]
    pub check: bool,
    #[serde(default)]
    pub compile_js: bool,
    #[serde(default)]
    pub compile_native: bool,
}

impl TierConfig {
    pub fn expects_pass(&self, tier: Tier) -> bool {
        match tier {
            Tier::Parse => self.parse,
            Tier::Check => self.check,
            Tier::CompileJs => self.compile_js,
            Tier::CompileNative => self.compile_native,
        }
    }
}

impl EcosystemManifest {
    /// Load manifest from TOML file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        toml::from_str(&content)
            .map_err(|e| format!("failed to parse {}: {e}", path.display()))
    }

    /// Discover all manifests in a directory.
    pub fn discover_all(dir: &Path) -> Vec<PathBuf> {
        let mut manifests = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    manifests.push(path);
                }
            }
        }
        manifests.sort();
        manifests
    }
}
