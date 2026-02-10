use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::Deserialize;

/// A phase tier for ecosystem validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
pub enum EcosystemPhase {
    /// Parse source files with parser level validation only.
    Parse = 1,
    /// Run resolve phase tasks.
    Resolve = 2,
    /// Run analyze phase tasks.
    Analyze = 3,
    /// Run lower phase tasks through optimize boundaries.
    Lower = 4,
}

impl EcosystemPhase {
    /// Return the stable lowercase phase name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Resolve => "resolve",
            Self::Analyze => "analyze",
            Self::Lower => "lower",
        }
    }

    /// Parse a phase name.
    pub fn from_name(value: &str) -> Option<Self> {
        match value {
            "parse" => Some(Self::Parse),
            "resolve" => Some(Self::Resolve),
            "analyze" => Some(Self::Analyze),
            "lower" => Some(Self::Lower),
            _ => None,
        }
    }

    /// Return all implemented phases in execution order.
    pub fn all() -> [Self; 4] {
        [Self::Parse, Self::Resolve, Self::Analyze, Self::Lower]
    }
}

/// A single ecosystem package manifest parsed from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct EcosystemManifest {
    /// Package metadata.
    pub package: PackageInfo,
    /// Base file discovery configuration used by all phases.
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    /// Per-phase workload settings for discovery and caps.
    #[serde(default)]
    pub workloads: WorkloadConfig,
}

/// Package metadata used for fetching and display.
#[derive(Debug, Clone, Deserialize)]
pub struct PackageInfo {
    /// Package name used as the checkouts directory name.
    pub name: String,
    /// Human-readable package description.
    #[serde(default)]
    pub description: String,
    /// Git repository URL.
    pub repo: String,
    /// Git ref (tag, branch, or commit), required for reproducibility.
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Primary source language used by this package.
    pub language: PackageLanguage,
    /// Optional category labels for filtering and reporting.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Primary source language for one ecosystem package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageLanguage {
    /// JavaScript first package.
    #[serde(alias = "javascript")]
    Js,
    /// TypeScript first package.
    #[serde(alias = "typescript")]
    Ts,
    /// Destack source package.
    #[serde(alias = "destack")]
    Ds,
}

/// Base file discovery configuration used by all phases.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DiscoveryConfig {
    /// Glob patterns to include when discovering files.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns to exclude when discovering files.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Per-phase workload configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct WorkloadConfig {
    /// Parse workload overrides.
    #[serde(default)]
    pub parse: PhaseWorkload,
    /// Resolve workload overrides.
    #[serde(default)]
    pub resolve: PhaseWorkload,
    /// Analyze workload overrides.
    #[serde(default)]
    pub analyze: PhaseWorkload,
    /// Lower workload overrides.
    #[serde(default)]
    pub lower: PhaseWorkload,
}

impl WorkloadConfig {
    /// Return workload settings for one phase.
    pub fn for_phase(&self, phase: EcosystemPhase) -> &PhaseWorkload {
        match phase {
            EcosystemPhase::Parse => &self.parse,
            EcosystemPhase::Resolve => &self.resolve,
            EcosystemPhase::Analyze => &self.analyze,
            EcosystemPhase::Lower => &self.lower,
        }
    }
}

/// Workload settings for one phase.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PhaseWorkload {
    /// Phase specific include patterns that replace base includes when set.
    #[serde(default)]
    pub include: Vec<String>,
    /// Phase specific exclude patterns added on top of base excludes.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Generic cap on discovered files for this phase.
    pub max_files: Option<usize>,
}

impl EcosystemManifest {
    /// Load a manifest from a TOML file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        toml::from_str(&content)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    /// Discover all manifest files in a directory.
    pub fn discover_all(dir: &Path) -> Vec<PathBuf> {
        let mut manifests = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "toml") {
                    manifests.push(path);
                }
            }
        }
        manifests.sort();
        manifests
    }
}
