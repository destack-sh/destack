//! Package manifest parsing for ecosystem tests.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Tier of ecosystem testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Parse = 1,
    Analyze = 2,
}

impl Tier {
    pub fn name(&self) -> &'static str {
        match self {
            Tier::Parse => "parse",
            Tier::Analyze => "analyze",
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
    /// Package name (used as directory name).
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Git repository URL.
    pub repo: String,
    /// Git ref (tag, branch, or commit) - required for reproducibility.
    #[serde(rename = "ref")]
    pub git_ref: String,
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
    pub parse: TierStatus,
    #[serde(default)]
    pub analyze: TierStatus,
}

/// Status of a tier - either passing (true), failing with reason, or not tested.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum TierStatus {
    /// Tier passes.
    Pass(bool),
    /// Tier fails with a reason.
    Fail {
        status: bool,
        reason: String,
    },
    /// Not tested yet.
    #[default]
    NotTested,
}

impl TierStatus {
    pub fn expects_pass(&self) -> bool {
        match self {
            TierStatus::Pass(v) => *v,
            TierStatus::Fail { status, .. } => *status,
            TierStatus::NotTested => false,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            TierStatus::Fail { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

impl TierConfig {
    pub fn get(&self, tier: Tier) -> &TierStatus {
        match tier {
            Tier::Parse => &self.parse,
            Tier::Analyze => &self.analyze,
        }
    }

    pub fn expects_pass(&self, tier: Tier) -> bool {
        self.get(tier).expects_pass()
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
