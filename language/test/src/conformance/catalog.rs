use std::path::{Path, PathBuf};
use std::{fmt, fs, io};

use serde::{Deserialize, Serialize};

/// The `suite.json` file name.
pub const SUITE_JSON_FILE_NAME: &str = "suite.json";

/// One conformance domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceDomain {
    /// The ECMAScript compatibility domain.
    Ecmascript,
    /// The Web API compatibility domain.
    Web,
    /// The Node API compatibility domain.
    Node,
    /// The formatter compatibility domain.
    Formatter,
}

impl ConformanceDomain {
    /// Return the stable domain identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ecmascript => "ecmascript",
            Self::Web => "web",
            Self::Node => "node",
            Self::Formatter => "formatter",
        }
    }
}

impl fmt::Display for ConformanceDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One conformance gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConformanceGate {
    /// The fast deterministic gate.
    Quick,
    /// The deeper local gate.
    Full,
    /// The slow scheduled gate.
    Nightly,
    /// The hardware or lab gate.
    Lab,
}

/// One conformance capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// One conformance environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Upstream source metadata for one corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamMetadata {
    /// The upstream repository URL.
    pub repo: String,
    /// The pinned upstream ref.
    #[serde(rename = "ref")]
    pub ref_: String,
}

/// One conformance suite metadata file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteMetadata {
    /// The stable suite identifier.
    pub id: String,
    /// The top level compatibility domain.
    pub domain: ConformanceDomain,
    /// The imported corpus name.
    pub corpus: String,
    /// The human readable suite title.
    pub title: String,
    /// The upstream source metadata.
    pub upstream: UpstreamMetadata,
    /// The gate where the suite belongs.
    pub gate: ConformanceGate,
    /// The environments required by the suite.
    pub environments: Vec<ConformanceEnvironment>,
    /// The supported capabilities for the suite.
    pub capabilities: Vec<ConformanceCapability>,
    /// The markdown files this suite should report into.
    pub report_targets: Vec<String>,
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
