use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{ConformanceCapability, ConformanceEnvironment};

/// The `expectations.json` file name.
pub const EXPECTATIONS_JSON_FILE_NAME: &str = "expectations.json";

/// One expectation status value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectationStatus {
    /// The case is a known failure.
    KnownFail,
    /// The case is intentionally ignored.
    Ignore,
    /// The case is blocked on environment availability.
    EnvBlocked,
    /// The case is flaky.
    Flaky,
    /// The case is manually managed.
    Manual,
}

/// One expectation entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectationEntry {
    /// The case pattern or identifier.
    pub pattern: String,
    /// The expectation status.
    pub status: ExpectationStatus,
    /// The reason for the expectation.
    pub reason: String,
    /// Optional capability filters.
    #[serde(default)]
    pub capabilities: Vec<ConformanceCapability>,
    /// Optional environment filters.
    #[serde(default)]
    pub environments: Vec<ConformanceEnvironment>,
}

/// One expectations file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectationSet {
    /// The expectation entries in declaration order.
    #[serde(default)]
    pub entries: Vec<ExpectationEntry>,
}

impl ExpectationSet {
    /// Load one expectations file.
    pub fn load(path: &Path) -> Result<Self, String> {
        // read the expectations file
        let content = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        // parse the json payload
        serde_json::from_str(&content)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    /// Return whether the set has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
