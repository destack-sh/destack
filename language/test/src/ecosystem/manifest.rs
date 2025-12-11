use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A tier of ecosystem testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Parse source files and ensure no errors are produced.
    Parse = 1,
    /// Analyze entrypoints and ensure no errors are produced.
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

/// A single ecosystem package manifest (parsed from TOML).
#[derive(Debug, Clone, Deserialize)]
pub struct EcosystemManifest {
    /// Package metadata.
    pub package: PackageInfo,
    /// File discovery configuration.
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    /// Per-tier expected status configuration.
    #[serde(default)]
    pub tiers: TierConfig,
}

/// Package metadata used for fetching and display.
#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    /// Package name (used as directory name).
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Git repository URL.
    pub repo: String,
    /// Git ref (tag, branch, or commit), required for reproducibility.
    #[serde(rename = "ref")]
    pub git_ref: String,
}

/// File discovery configuration for ecosystem tiers.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiscoveryConfig {
    /// Glob patterns to include when discovering files.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns to exclude when discovering files.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Expected status configuration for each tier.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TierConfig {
    /// Expected status of the parse tier.
    #[serde(default)]
    pub parse: TierStatus,
    /// Expected status of the analyze tier.
    #[serde(default)]
    pub analyze: TierStatus,
}

/// Status of a tier: either passing (true), failing with a reason, or not tested.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum TierStatus {
    /// Tier passes.
    /// `true` means the tier is expected to pass, and `false` means it is expected to fail.
    Pass(bool),
    /// Tier fails with a reason.
    Fail {
        /// Whether the tier is expected to pass.
        status: bool,
        /// Human-readable reason for the expected status.
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
        toml::from_str(&content).map_err(|e| format!("failed to parse {}: {e}", path.display()))
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
