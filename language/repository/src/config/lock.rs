use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Dependency;

/// Destack lock document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct DestackLock {
    /// Lock document format version.
    pub version: u32,
    /// Root package name.
    pub root: Option<String>,
    /// Locked packages keyed by package import name.
    pub packages: IndexMap<String, PackageLock>,
}

/// Locked package source and dependency record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PackageLock {
    /// Package name read from the locked manifest.
    pub name: String,
    /// Package version read from the locked manifest.
    pub version: Option<String>,
    /// Resolved source locator.
    pub source: SourceLock,
    /// Hash of the package source tree.
    pub source_hash: String,
    /// Hash of the package manifest.
    pub manifest_hash: String,
    /// Locked dependency declarations.
    pub dependencies: IndexMap<String, Dependency>,
    /// Whether this package is an editable source.
    pub editable: bool,
}

/// Locked source locator.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum SourceLock {
    /// Package resolved from a workspace member.
    Workspace {
        /// Workspace relative package path.
        path: String,
    },
    /// Package resolved from a registry.
    Registry {
        /// Registry name.
        registry: Option<String>,
        /// Exact package version.
        version: String,
    },
    /// Package resolved from a local path.
    Path {
        /// Package path.
        path: String,
    },
    /// Package resolved from one Git repository.
    Git {
        /// Repository URL.
        url: String,
        /// Exact commit revision.
        rev: String,
        /// Repository subdirectory containing the package.
        path: Option<String>,
    },
}

impl Default for SourceLock {
    fn default() -> Self {
        Self::Workspace {
            path: ".".to_string(),
        }
    }
}
