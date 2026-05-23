use std::path::PathBuf;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::{ConditionGate, ConditionSet};

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
    pub when: ConditionPredicate,
    /// Dependency declarations enabled when the predicate matches.
    pub dependencies: IndexMap<String, Dependency>,
}

/// Predicate selecting one active condition set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ConditionPredicate {
    /// Named active condition.
    Alias(String),
    /// Inline condition gate.
    Gate(ConditionGate),
}

impl Default for ConditionPredicate {
    fn default() -> Self {
        Self::Gate(ConditionGate::default())
    }
}

impl ConditionPredicate {
    /// Return whether this predicate matches one active condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        match self {
            Self::Alias(name) => {
                conditions.contains_mode(name)
                    || conditions.contains_role(name)
                    || conditions.contains_feature(name)
                    || conditions.contains_tag(name)
            }
            Self::Gate(gate) => gate.matches(conditions),
        }
    }
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
