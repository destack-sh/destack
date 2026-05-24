use std::path::PathBuf;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::ConditionRef;

/// Package dependency declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum Dependency {
    /// Package resolved from the current workspace.
    Workspace,
    /// Package resolved from one local package path.
    Path {
        /// Package path.
        path: PathBuf,
    },
    /// Package resolved from one Git repository.
    Git {
        /// Repository URL.
        url: String,
        /// Repository subdirectory containing the package.
        path: Option<PathBuf>,
        /// Exact commit revision.
        rev: Option<String>,
        /// Git tag selector.
        tag: Option<String>,
        /// Git branch selector.
        branch: Option<String>,
    },
}

/// Dependency declarations guarded by one active condition predicate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalDependencies {
    /// Condition predicate enabling these dependencies.
    pub when: ConditionRef,
    /// Dependency declarations enabled when the predicate matches.
    pub dependencies: IndexMap<String, Dependency>,
}

/// Patch file applied to one resolved package.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PackagePatch {
    /// Patch file path.
    pub path: PathBuf,
}
