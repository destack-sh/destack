use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(feature = "schema")]
use std::borrow::Cow;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::config::ConditionRef;

/// The specifier of a workspace dependency.
const WORKSPACE_SPECIFIER: &str = "workspace:*";
/// The specifier prefix of a local path dependency.
const FILE_PREFIX: &str = "file:";
/// The specifier prefix of a git dependency.
const GIT_PREFIX: &str = "git+";
/// Characters that make a registry specifier a version range instead of one exact version.
const VERSION_RANGE_CHARACTERS: &[char] = &[' ', '^', '~', '<', '>', '=', '*', '|'];

/// Package dependency declaration, written as one specifier string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dependency {
    /// Package resolved from the current workspace, written `workspace:*`.
    Workspace,
    /// Package resolved from the package registry, written as its exact version.
    Registry {
        /// Exact package version.
        version: String,
    },
    /// Package resolved from one local package path, written `file:<path>`.
    Path {
        /// Package path.
        path: PathBuf,
    },
    /// Package resolved from one Git repository, written `git+<url>#<field>=<value>&...`.
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

impl FromStr for Dependency {
    type Err = String;

    /// Parse one dependency specifier.
    fn from_str(specifier: &str) -> Result<Self, Self::Err> {
        let is_range = !specifier.starts_with(|character: char| character.is_ascii_digit())
            || specifier.contains(VERSION_RANGE_CHARACTERS)
            || specifier
                .split('.')
                .any(|part| part.eq_ignore_ascii_case("x"));

        // resolve a workspace package
        if specifier == WORKSPACE_SPECIFIER {
            Ok(Self::Workspace)
        }
        // read a local package path
        else if let Some(path) = specifier.strip_prefix(FILE_PREFIX) {
            Ok(Self::Path {
                path: PathBuf::from(path),
            })
        }
        // read a git repository with optional selectors
        else if let Some(source) = specifier.strip_prefix(GIT_PREFIX) {
            Self::parse_git(source)
        }
        // reject an empty specifier or an unknown scheme
        else if specifier.is_empty() || specifier.contains(':') {
            Err(format!("unknown dependency specifier '{specifier}'"))
        }
        // reject a version range or tag
        else if is_range {
            Err(format!(
                "dependency version '{specifier}' must be one exact version"
            ))
        }
        // pin one registry version
        else {
            Ok(Self::Registry {
                version: specifier.to_string(),
            })
        }
    }
}

impl Dependency {
    /// Parse one git specifier after its `git+` prefix.
    fn parse_git(source: &str) -> Result<Self, String> {
        let (url, selectors) = source.split_once('#').unwrap_or((source, ""));
        let mut path = None;
        let mut rev = None;
        let mut tag = None;
        let mut branch = None;

        // read each `field=value` selector
        for selector in selectors.split('&').filter(|selector| !selector.is_empty()) {
            let Some((field, value)) = selector.split_once('=') else {
                return Err(format!("git selector '{selector}' must be 'field=value'"));
            };
            let value = value.to_string();
            match field {
                "path" => path = Some(PathBuf::from(value)),
                "rev" => rev = Some(value),
                "tag" => tag = Some(value),
                "branch" => branch = Some(value),
                _ => return Err(format!("unknown git selector '{field}'")),
            }
        }

        Ok(Self::Git {
            url: url.to_string(),
            path,
            rev,
            tag,
            branch,
        })
    }
}

impl fmt::Display for Dependency {
    /// Write this dependency as its specifier string.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace => formatter.write_str(WORKSPACE_SPECIFIER),
            Self::Registry { version } => formatter.write_str(version),
            Self::Path { path } => write!(formatter, "{FILE_PREFIX}{}", path.display()),
            Self::Git {
                url,
                path,
                rev,
                tag,
                branch,
            } => {
                write!(formatter, "{GIT_PREFIX}{url}")?;

                // write the present selectors in field order
                let path = path.as_ref().map(|path| path.display().to_string());
                let selectors = [
                    ("path", path.as_ref()),
                    ("rev", rev.as_ref()),
                    ("tag", tag.as_ref()),
                    ("branch", branch.as_ref()),
                ];
                let mut separator = '#';
                for (field, value) in selectors {
                    if let Some(value) = value {
                        write!(formatter, "{separator}{field}={value}")?;
                        separator = '&';
                    }
                }

                Ok(())
            }
        }
    }
}

impl Serialize for Dependency {
    /// Write this dependency as its specifier string.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Dependency {
    /// Read one dependency from its specifier string.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let specifier = String::deserialize(deserializer)?;

        specifier.parse().map_err(de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Dependency {
    /// Return the schema name of a dependency specifier.
    fn schema_name() -> Cow<'static, str> {
        "Dependency".into()
    }

    /// Describe a dependency as one specifier string.
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        String::json_schema(generator)
    }
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
