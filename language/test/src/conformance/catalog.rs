use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use serde::{Deserialize, Serialize};

use super::{StatusSet, status_json_path_for_dir};

/// The `suite.json` file name.
pub const SUITE_JSON_FILE_NAME: &str = "suite.json";

/// One conformance domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceDomain {
    /// The ECMA conformance domain.
    Ecma,
    /// The formatter conformance domain.
    Formatter,
}

impl ConformanceDomain {
    /// Return the stable domain identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecma => "ecma",
            Self::Formatter => "formatter",
        }
    }
}

impl fmt::Display for ConformanceDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One conformance capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceCapability {
    /// The parsing capability.
    Parse,
    /// The checking capability.
    Check,
    /// The execution capability.
    Run,
    /// The formatting capability.
    Format,
    /// The emit capability.
    Emit,
    /// The query capability.
    Query,
    /// The LSP capability.
    Lsp,
}

impl ConformanceCapability {
    /// Return the stable capability identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Check => "check",
            Self::Run => "run",
            Self::Format => "format",
            Self::Emit => "emit",
            Self::Query => "query",
            Self::Lsp => "lsp",
        }
    }
}

impl fmt::Display for ConformanceCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One conformance environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceEnvironment {
    /// A hostless environment.
    Hostless,
    /// A filesystem backed environment.
    Fs,
    /// A network backed environment.
    Network,
    /// A worker backed environment.
    Worker,
    /// A GPU backed environment.
    Gpu,
    /// An offline audio environment.
    AudioOffline,
    /// A serial device environment.
    DeviceSerial,
    /// A USB device environment.
    DeviceUsb,
    /// A bluetooth environment.
    Bluetooth,
    /// A manual or special environment.
    Manual,
}

impl ConformanceEnvironment {
    /// Return the stable environment identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hostless => "hostless",
            Self::Fs => "fs",
            Self::Network => "network",
            Self::Worker => "worker",
            Self::Gpu => "gpu",
            Self::AudioOffline => "audio-offline",
            Self::DeviceSerial => "device-serial",
            Self::DeviceUsb => "device-usb",
            Self::Bluetooth => "bluetooth",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for ConformanceEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One origin kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OriginKind {
    /// A git backed origin.
    Git,
}

impl OriginKind {
    /// Return the stable origin kind identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
        }
    }
}

impl fmt::Display for OriginKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Origin source metadata for one suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OriginMetadata {
    /// The origin transport kind.
    pub kind: OriginKind,
    /// The origin repository URL.
    pub repo: String,
    /// The pinned origin ref.
    #[serde(rename = "ref")]
    pub ref_: String,
}

impl OriginMetadata {
    /// Validate one origin payload for one suite.
    fn validate(&self, suite_id: &str, label: &str, directory: &Path) -> Result<(), String> {
        if self.repo.trim().is_empty() || self.ref_.trim().is_empty() {
            return Err(format!(
                "suite '{}' in {} has incomplete {label} origin metadata",
                suite_id,
                directory.display()
            ));
        }

        Ok(())
    }
}

/// One fetch entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FetchEntryKind {
    /// One fetched source entry.
    Source,
    /// One translated local entry.
    Translated,
    /// One manual local entry.
    Manual,
}

/// One fetch file entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchFileEntry {
    /// The source-relative path for one fetched file.
    #[serde(default)]
    pub source: String,
    /// The case-relative target path for one fetched file.
    #[serde(default)]
    pub target: String,
    /// The case-relative local file path.
    #[serde(default)]
    pub path: String,
}

/// One fetch entry in one suite manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchEntry {
    /// The fetch entry kind.
    pub kind: FetchEntryKind,
    /// The human-readable fetch label.
    #[serde(default)]
    pub label: String,
    /// The fetched source version label.
    #[serde(default)]
    pub version: String,
    /// The fetch layout mode.
    #[serde(default)]
    pub layout: String,
    /// The cached source root for copied files.
    #[serde(default)]
    pub root: String,
    /// The sparse checkout paths for fetched sources.
    #[serde(default)]
    pub paths: Vec<String>,
    /// The path patterns to keep after fetching one source entry.
    #[serde(default)]
    pub include: Vec<String>,
    /// The path patterns to drop after fetching one source entry.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// The declared file mappings.
    #[serde(default)]
    pub files: Vec<FetchFileEntry>,
}

/// One fetch manifest embedded in one suite file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchMetadata {
    /// The fetch entries for one suite.
    #[serde(default)]
    pub entries: Vec<FetchEntry>,
}

/// One conformance suite metadata file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SuiteMetadata {
    /// The stable suite identifier.
    pub id: String,
    /// The top level conformance domain.
    pub domain: ConformanceDomain,
    /// The imported suite name.
    #[serde(alias = "corpus")]
    pub suite: String,
    /// The human readable suite title.
    pub title: String,
    /// The origin source metadata.
    pub origin: OriginMetadata,
    /// The optional fetch metadata for the suite case tree.
    #[serde(default)]
    pub fetch: FetchMetadata,
}

impl SuiteMetadata {
    /// Load one suite metadata file.
    pub fn load(path: &Path) -> Result<Self, String> {
        // read the suite file
        let content = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        // parse the json payload
        serde_json::from_str(&content)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    /// Validate one metadata payload against one suite directory.
    pub fn validate(&self, directory: &Path) -> Result<(), String> {
        // validate the stable suite id
        let expected_id = format!("{}.{}", self.domain, self.suite);
        if self.id != expected_id {
            return Err(format!(
                "suite '{}' in {} should use id '{}'",
                self.id,
                directory.display(),
                expected_id
            ));
        }

        // validate the expected suite directory
        let actual_suite = directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if actual_suite != self.suite {
            return Err(format!(
                "suite '{}' lives in suite directory '{}' instead of '{}'",
                self.id, actual_suite, self.suite
            ));
        }

        // validate the non-empty string fields
        if self.title.trim().is_empty() {
            return Err(format!(
                "suite '{}' in {} has an empty title",
                self.id,
                directory.display()
            ));
        }

        self.origin.validate(&self.id, "primary", directory)?;

        Ok(())
    }
}

/// One fully loaded conformance suite record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceSuiteRecord {
    /// The suite directory under the conformance fixtures root.
    pub directory: PathBuf,
    /// The parsed suite metadata.
    pub suite: SuiteMetadata,
    /// The parsed status metadata.
    pub statuses: StatusSet,
}

/// One loaded conformance catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceCatalog {
    /// The loaded suite records in stable order.
    pub suites: Vec<ConformanceSuiteRecord>,
}

impl ConformanceCatalog {
    /// Load one catalog from the provided fixtures root.
    pub fn load(root: &Path) -> Result<Self, String> {
        let suite_paths = discover_suite_metadata_paths(root)
            .map_err(|error| format!("failed to discover {}: {error}", root.display()))?;

        let mut suites = Vec::with_capacity(suite_paths.len());
        let mut ids = BTreeSet::new();

        // load each declared suite
        for suite_path in suite_paths {
            let Some(directory) = suite_path.parent() else {
                return Err(format!(
                    "suite metadata path '{}' has no parent directory",
                    suite_path.display()
                ));
            };

            let suite = SuiteMetadata::load(&suite_path)?;
            suite.validate(directory)?;

            if !ids.insert(suite.id.clone()) {
                return Err(format!("duplicate conformance suite id '{}'", suite.id));
            }

            let status_path = status_json_path_for_dir(directory);
            let statuses = StatusSet::load(&status_path)?;

            suites.push(ConformanceSuiteRecord {
                directory: directory.to_path_buf(),
                suite,
                statuses,
            });
        }

        // keep the catalog deterministic
        suites.sort_by(|left, right| left.suite.id.cmp(&right.suite.id));

        Ok(Self { suites })
    }
}

/// Return the `suite.json` path for one suite directory.
pub fn suite_json_path_for_dir(directory: &Path) -> PathBuf {
    directory.join(SUITE_JSON_FILE_NAME)
}

/// Discover all `suite.json` files under one root.
pub fn discover_suite_metadata_paths(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_suite_metadata_paths(root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

/// Collect all `suite.json` files under one directory tree.
fn collect_suite_metadata_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    // stop when the directory is missing
    if !directory.exists() {
        return Ok(());
    }

    // recurse through the tree
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();

        // register matching suite files
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == SUITE_JSON_FILE_NAME)
        {
            paths.push(path);
            continue;
        }

        // keep walking into child directories
        if path.is_dir() {
            collect_suite_metadata_paths(&path, paths)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{FetchMetadata, SuiteMetadata};
    use crate::conformance::{ConformanceDomain, OriginKind, OriginMetadata};

    #[test]
    fn test_validate_accepts_matching_directory_layout() {
        let metadata = SuiteMetadata {
            id: "ecma.test262".to_string(),
            domain: ConformanceDomain::Ecma,
            suite: "test262".to_string(),
            title: "ECMA Test262".to_string(),
            origin: OriginMetadata {
                kind: OriginKind::Git,
                repo: "https://github.com/tc39/test262".to_string(),
                ref_: "main".to_string(),
            },
            fetch: FetchMetadata::default(),
        };

        let result = metadata.validate(Path::new("fixtures/conformance/test262"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rejects_mismatched_ids() {
        let metadata = SuiteMetadata {
            id: "wrong.id".to_string(),
            domain: ConformanceDomain::Ecma,
            suite: "test262".to_string(),
            title: "ECMA Test262".to_string(),
            origin: OriginMetadata {
                kind: OriginKind::Git,
                repo: "https://github.com/tc39/test262".to_string(),
                ref_: "main".to_string(),
            },
            fetch: FetchMetadata::default(),
        };

        let result = metadata.validate(Path::new("fixtures/conformance/test262"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_accepts_flat_suite_layout() {
        let metadata = SuiteMetadata {
            id: "formatter.oxfmt".to_string(),
            domain: ConformanceDomain::Formatter,
            suite: "oxfmt".to_string(),
            title: "Formatter Oxfmt".to_string(),
            origin: OriginMetadata {
                kind: OriginKind::Git,
                repo: "https://github.com/oxc-project/oxc".to_string(),
                ref_: "main".to_string(),
            },
            fetch: FetchMetadata::default(),
        };

        let result = metadata.validate(Path::new("fixtures/conformance/oxfmt"));
        assert!(result.is_ok());
    }
}
