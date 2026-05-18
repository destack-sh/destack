use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::ConditionGate;

/// Package dependency declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum Dependency {
    /// Package resolved from the Destack package registry.
    Registry {
        /// Version requirement.
        version: String,
    },
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
        /// Optional branch, tag, or revision selector.
        #[serde(rename = "ref")]
        reference: Option<String>,
    },
}

impl Dependency {
    /// Validate one dependency declaration.
    pub fn validate(&self, name: &str) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("dependency name must not be empty".to_string());
        }

        match self {
            Self::Registry { version } => validate_dependency_text(version, name, "version"),
            Self::Workspace => Ok(()),
            Self::Path { path } => validate_dependency_path(path, name),
            Self::Git { url, reference } => {
                validate_dependency_text(url, name, "url")?;

                if let Some(reference) = reference {
                    validate_dependency_text(reference, name, "ref")?;
                }

                Ok(())
            }
        }
    }
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
    /// Named condition alias.
    Alias(String),
    /// Inline condition gate.
    Gate(ConditionGate),
}

impl Default for ConditionPredicate {
    fn default() -> Self {
        Self::Gate(ConditionGate::default())
    }
}

/// Package registry declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    /// Registry base URL.
    pub url: String,
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

/// Validate dependency declarations.
pub(crate) fn validate_dependency_map(
    dependencies: Option<&IndexMap<String, Dependency>>,
) -> Result<(), String> {
    if let Some(dependencies) = dependencies {
        for (name, dependency) in dependencies {
            dependency.validate(name)?;
        }
    }

    Ok(())
}

/// Validate that one dependency path is present.
fn validate_dependency_path(path: &Path, name: &str) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        Err(format!("dependency '{name}' path must not be empty"))
    } else {
        Ok(())
    }
}

/// Validate that one dependency text field is present after trimming.
fn validate_dependency_text(text: &str, name: &str, field: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        Err(format!("dependency '{name}' {field} must not be empty"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_registry_dependency() {
        let dependency = Dependency::Registry {
            version: "^1.2.0".to_string(),
        };

        assert_eq!(dependency.validate("shared"), Ok(()));
    }

    #[test]
    fn test_validate_workspace_dependency() {
        let dependency = Dependency::Workspace;

        assert_eq!(dependency.validate("shared"), Ok(()));
    }

    #[test]
    fn test_validate_git_reference() {
        let dependency = Dependency::Git {
            url: "https://example.com/shared.git".to_string(),
            reference: Some("main".to_string()),
        };

        assert_eq!(dependency.validate("shared"), Ok(()));
    }

    #[test]
    fn test_reject_empty_dependency_name() {
        let dependency = Dependency::Workspace;

        assert_eq!(
            dependency.validate(""),
            Err("dependency name must not be empty".to_string())
        );
    }
}
