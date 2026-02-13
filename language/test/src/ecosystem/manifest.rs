use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::Deserialize;

/// A phase tier for ecosystem validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
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

/// A support tier target for one ecosystem package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum EcosystemSupportTier {
    /// Parse support.
    #[serde(rename = "T0", alias = "t0")]
    T0,
    /// Resolve support.
    #[serde(rename = "T1", alias = "t1")]
    T1,
    /// Analyze support.
    #[serde(rename = "T2", alias = "t2")]
    T2,
    /// Lower support.
    #[serde(rename = "T3", alias = "t3")]
    T3,
    /// Run support.
    #[serde(rename = "T4", alias = "t4")]
    T4,
    /// Test support.
    #[serde(rename = "T5", alias = "t5")]
    T5,
}

impl EcosystemSupportTier {
    /// Return the stable uppercase support tier label.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::T0 => "T0",
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
            Self::T5 => "T5",
        }
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
    /// Compiler options overrides for ecosystem compatibility.
    #[serde(default)]
    pub compiler_options: CompilerOptionsConfig,
    /// TSC configuration used for TypeScript parity checks.
    #[serde(default)]
    pub tsc: TscConfig,
    /// Per-phase workload settings for discovery and caps.
    #[serde(default)]
    pub workloads: WorkloadConfig,
    /// Patch metadata for support markers.
    #[serde(default)]
    pub patch: PatchConfig,
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
    /// Intended ecosystem support tier target.
    pub target_tier: Option<EcosystemSupportTier>,
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

/// Parser behavior overrides.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CompilerOptionsConfig {
    /// Parse `.js`/`.mjs`/`.cjs` files in JSX mode.
    pub js_as_jsx: Option<bool>,
}

impl CompilerOptionsConfig {
    /// Return whether plain js files should parse in jsx mode.
    pub fn js_as_jsx(&self) -> bool {
        self.js_as_jsx.unwrap_or(false)
    }
}

/// TSC execution mode for ecosystem phase validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EcosystemTscMode {
    /// Disable tsc checks.
    Off,
    /// Run tsc checks only after Destack reports errors.
    #[default]
    OnFailure,
    /// Always run tsc checks.
    Always,
}

/// TSC tool selection for ecosystem phase validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "lowercase")]
pub enum EcosystemTscTool {
    /// Prefer `tsgo` and fall back to `tsc`.
    #[default]
    Auto,
    /// Use `tsgo` directly.
    Tsgo,
    /// Use `tsc` directly.
    Tsc,
}

/// TSC configuration for one ecosystem package.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TscConfig {
    /// Enable or disable tsc checks for this package.
    pub enabled: Option<bool>,
    /// TSC execution mode.
    pub mode: Option<EcosystemTscMode>,
    /// TSC tool selection.
    pub tool: Option<EcosystemTscTool>,
    /// Phase allowlist for tsc execution.
    #[serde(default)]
    pub phases: Vec<EcosystemPhase>,
}

impl TscConfig {
    /// Return whether tsc checks are enabled for one package language.
    pub fn enabled_for_language(&self, language: PackageLanguage) -> bool {
        self.enabled
            .unwrap_or(matches!(language, PackageLanguage::Ts))
    }

    /// Return the tsc execution mode with default fallback.
    pub fn mode(&self) -> EcosystemTscMode {
        self.mode.unwrap_or_default()
    }

    /// Return the tsc tool with default fallback.
    pub fn tool(&self) -> EcosystemTscTool {
        self.tool.unwrap_or_default()
    }

    /// Return whether one phase is eligible for tsc execution.
    pub fn includes_phase(&self, phase: EcosystemPhase) -> bool {
        if self.phases.is_empty() {
            return matches!(phase, EcosystemPhase::Resolve | EcosystemPhase::Analyze);
        }

        self.phases.contains(&phase)
    }
}

/// Patch metadata for ecosystem compatibility markers.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PatchConfig {
    /// Mark packages that need dependency patching or replacement.
    pub dependency_replacement: Option<bool>,
}

impl PatchConfig {
    /// Return whether this package needs dependency patching or replacement.
    pub fn dependency_replacement(&self) -> bool {
        self.dependency_replacement.unwrap_or(false)
    }
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

#[cfg(test)]
mod tests {
    use super::{
        EcosystemManifest, EcosystemPhase, EcosystemSupportTier, EcosystemTscMode,
        EcosystemTscTool, PackageLanguage,
    };

    fn parse_manifest(content: &str) -> EcosystemManifest {
        toml::from_str(content).unwrap()
    }

    #[test]
    fn test_parse_compiler_options_js_as_jsx() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"

[compiler_options]
js_as_jsx = true
"#,
        );

        assert!(manifest.compiler_options.js_as_jsx());
    }

    #[test]
    fn test_parse_compiler_options_js_as_jsx_default_false() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"
"#,
        );

        assert!(!manifest.compiler_options.js_as_jsx());
    }

    #[test]
    fn test_parse_patch_dependency_replacement() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"

[patch]
dependency_replacement = true
"#,
        );

        assert!(manifest.patch.dependency_replacement());
    }

    #[test]
    fn test_parse_patch_dependency_replacement_default_false() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"
"#,
        );

        assert!(!manifest.patch.dependency_replacement());
    }

    #[test]
    fn test_parse_package_target_tier() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"
target_tier = "T3"
"#,
        );

        assert_eq!(manifest.package.target_tier, Some(EcosystemSupportTier::T3));
    }

    #[test]
    fn test_parse_package_target_tier_default_none() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"
"#,
        );

        assert_eq!(manifest.package.target_tier, None);
    }

    #[test]
    fn test_parse_tsc_configuration() {
        let manifest = parse_manifest(
            r#"
[package]
name = "demo"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"

[tsc]
enabled = true
mode = "always"
tool = "tsgo"
phases = ["resolve", "analyze"]
"#,
        );

        assert!(manifest.tsc.enabled_for_language(PackageLanguage::Ts));
        assert_eq!(manifest.tsc.mode(), EcosystemTscMode::Always);
        assert_eq!(manifest.tsc.tool(), EcosystemTscTool::Tsgo);
        assert!(manifest.tsc.includes_phase(EcosystemPhase::Resolve));
        assert!(manifest.tsc.includes_phase(EcosystemPhase::Analyze));
        assert!(!manifest.tsc.includes_phase(EcosystemPhase::Lower));
    }

    #[test]
    fn test_parse_tsc_defaults_for_package_language() {
        let ts_manifest = parse_manifest(
            r#"
[package]
name = "demo-ts"
repo = "https://example.com/repo.git"
ref = "main"
language = "ts"
"#,
        );
        let js_manifest = parse_manifest(
            r#"
[package]
name = "demo-js"
repo = "https://example.com/repo.git"
ref = "main"
language = "js"
"#,
        );

        assert!(ts_manifest.tsc.enabled_for_language(PackageLanguage::Ts));
        assert!(!js_manifest.tsc.enabled_for_language(PackageLanguage::Js));
        assert_eq!(ts_manifest.tsc.mode(), EcosystemTscMode::OnFailure);
        assert_eq!(ts_manifest.tsc.tool(), EcosystemTscTool::Auto);
        assert!(ts_manifest.tsc.includes_phase(EcosystemPhase::Resolve));
        assert!(ts_manifest.tsc.includes_phase(EcosystemPhase::Analyze));
        assert!(!ts_manifest.tsc.includes_phase(EcosystemPhase::Parse));
    }
}
