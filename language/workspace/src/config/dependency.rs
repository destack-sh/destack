use std::path::PathBuf;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Package dependency declarations keyed by package specifier.
pub type DependencyMap = IndexMap<String, DependencyOptions>;

/// Package dependency declarations keyed by package specifier.
pub type DependencyJsonMap = IndexMap<String, DependencyOptionsJson>;

/// Package dependency declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependencyOptions {
    /// Dependency source.
    pub source: DependencySource,
}

impl DependencyOptions {
    /// Convert from one dependency JSON declaration.
    pub fn from_json(json: &DependencyOptionsJson) -> Self {
        match json {
            DependencyOptionsJson::Requirement(requirement) => Self {
                source: dependency_source_from_requirement(requirement),
            },
            DependencyOptionsJson::Options(options) => Self {
                source: options.source(),
            },
        }
    }
}

/// Package dependency source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencySource {
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
        reference: Option<String>,
    },
}

/// Dependency declaration accepted by `destack.json`.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DependencyOptionsJson {
    /// Registry version or workspace requirement shorthand.
    Requirement(String),
    /// Structured dependency declaration.
    Options(DependencySourceJson),
}

impl DependencyOptionsJson {
    /// Validate one dependency declaration.
    pub fn validate(&self, name: &str) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("dependency name must not be empty".to_string());
        }

        match self {
            Self::Requirement(requirement) => {
                validate_dependency_text(requirement, name, "requirement")
            }
            Self::Options(options) => options.validate(name),
        }
    }
}

/// Structured dependency source accepted by `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DependencySourceJson {
    /// Registry version requirement.
    pub version: Option<String>,
    /// Whether this dependency resolves from the current workspace.
    pub workspace: Option<bool>,
    /// Local package path.
    pub path: Option<String>,
    /// Git repository URL.
    pub git: Option<String>,
    /// Git branch, tag, or revision selector.
    #[serde(rename = "ref")]
    pub reference: Option<String>,
}

impl DependencySourceJson {
    /// Validate one structured dependency declaration.
    pub fn validate(&self, name: &str) -> Result<(), String> {
        let mut sources = 0usize;

        if self.version.is_some() {
            sources += 1;
        }

        if self.workspace.unwrap_or(false) {
            sources += 1;
        }

        if self.path.is_some() {
            sources += 1;
        }

        if self.git.is_some() {
            sources += 1;
        }

        if sources != 1 {
            return Err(format!(
                "dependency '{name}' must declare exactly one source"
            ));
        }

        if let Some(version) = &self.version {
            validate_dependency_text(version, name, "version")?;
        }

        if let Some(path) = &self.path {
            validate_dependency_text(path, name, "path")?;
        }

        if let Some(git) = &self.git {
            validate_dependency_text(git, name, "git")?;
        }

        if let Some(reference) = &self.reference {
            validate_dependency_text(reference, name, "ref")?;
        }

        if self.reference.is_some() && self.git.is_none() {
            return Err(format!("dependency '{name}' ref requires git"));
        }

        Ok(())
    }

    /// Return the normalized dependency source.
    fn source(&self) -> DependencySource {
        if let Some(version) = &self.version {
            return DependencySource::Registry {
                version: version.clone(),
            };
        }

        if self.workspace.unwrap_or(false) {
            return DependencySource::Workspace;
        }

        if let Some(path) = &self.path {
            return DependencySource::Path {
                path: PathBuf::from(path),
            };
        }

        if let Some(git) = &self.git {
            return DependencySource::Git {
                url: git.clone(),
                reference: self.reference.clone(),
            };
        }

        unreachable!("dependency source should be validated before normalization")
    }
}

/// Convert dependency JSON declarations to normalized options.
pub fn dependency_options_from_json(json: &Option<DependencyJsonMap>) -> DependencyMap {
    json.as_ref()
        .map(|dependencies| {
            dependencies
                .iter()
                .map(|(name, dependency)| {
                    let dependency = DependencyOptions::from_json(dependency);

                    (name.clone(), dependency)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Validate dependency JSON declarations.
pub fn validate_dependency_json_map(
    dependencies: Option<&DependencyJsonMap>,
) -> Result<(), String> {
    if let Some(dependencies) = dependencies {
        for (name, dependency) in dependencies {
            dependency.validate(name)?;
        }
    }

    Ok(())
}

/// Validate that one dependency text field is present after trimming.
fn validate_dependency_text(text: &str, name: &str, field: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        Err(format!("dependency '{name}' {field} must not be empty"))
    } else {
        Ok(())
    }
}

/// Return a dependency source from one string requirement.
fn dependency_source_from_requirement(requirement: &str) -> DependencySource {
    if requirement == "workspace" || requirement.starts_with("workspace:") {
        DependencySource::Workspace
    } else {
        DependencySource::Registry {
            version: requirement.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry_requirement() {
        let json = DependencyOptionsJson::Requirement("^1.2.0".to_string());
        let dependency = DependencyOptions::from_json(&json);

        assert_eq!(
            dependency.source,
            DependencySource::Registry {
                version: "^1.2.0".to_string()
            }
        );
    }

    #[test]
    fn test_parse_workspace_requirement() {
        let json = DependencyOptionsJson::Requirement("workspace".to_string());
        let dependency = DependencyOptions::from_json(&json);

        assert_eq!(dependency.source, DependencySource::Workspace);
    }

    #[test]
    fn test_reject_multiple_sources() {
        let json = DependencyOptionsJson::Options(DependencySourceJson {
            version: Some("^1.2.0".to_string()),
            workspace: Some(true),
            ..DependencySourceJson::default()
        });

        assert_eq!(
            json.validate("shared"),
            Err("dependency 'shared' must declare exactly one source".to_string())
        );
    }

    #[test]
    fn test_reject_git_ref_without_git() {
        let json = DependencyOptionsJson::Options(DependencySourceJson {
            version: Some("^1.2.0".to_string()),
            reference: Some("main".to_string()),
            ..DependencySourceJson::default()
        });

        assert_eq!(
            json.validate("shared"),
            Err("dependency 'shared' ref requires git".to_string())
        );
    }
}
