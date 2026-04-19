use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::ProfileConfigJson;

use super::cache::CacheJson;
use super::compiler::CompilerOptionsJson;
use super::daemon::DaemonJson;
use super::environment::EnvironmentJson;
use super::formatter::FormatterJson;
use super::linter::LinterJson;
use super::mode::ModeJson;
use super::runtime::RuntimeConfigJson;
use super::target::TargetJson;
use super::watch::WatchJson;
use super::workspace::WorkspaceJson;

/// Destack config JSON, usually from `destack.json`.
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DestackJson {
    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
    /// Whether the package is private.
    #[serde(rename = "private")]
    pub r#private: Option<bool>,
    /// Package description.
    pub description: Option<String>,
    /// Package license identifier.
    pub license: Option<String>,
    /// Package repository metadata.
    pub repository: Option<Value>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package keywords.
    pub keywords: Option<Vec<String>>,
    /// Preferred package manager string.
    pub package_manager: Option<String>,
    /// Package module type.
    #[serde(rename = "type")]
    pub module_type: Option<String>,
    /// Package engines.
    pub engines: Option<IndexMap<String, String>>,
    /// Package exports map.
    pub exports: Option<Value>,
    /// Package imports map.
    pub imports: Option<IndexMap<String, Value>>,
    /// Runtime dependencies.
    pub dependencies: Option<IndexMap<String, String>>,
    /// Development dependencies.
    pub dev_dependencies: Option<IndexMap<String, String>>,
    /// Peer dependencies.
    pub peer_dependencies: Option<IndexMap<String, String>>,
    /// Optional dependencies.
    pub optional_dependencies: Option<IndexMap<String, String>>,
    /// Repository wide workspace membership.
    pub workspace: Option<WorkspaceJson>,
    /// Extends other Destack configs or tsconfig files by path.
    pub extends: Option<ExtendsFieldJson>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    #[serde(default, alias = "compilerOptions")]
    pub compiler: CompilerOptionsJson,
    /// Runtime options.
    #[serde(default)]
    pub runtime: RuntimeConfigJson,
    /// Formatter options.
    #[serde(default, alias = "formatterOptions")]
    pub formatter: FormatterJson,
    /// Linter options.
    #[serde(default, alias = "linterOptions")]
    pub linter: LinterJson,
    /// Cache options.
    #[serde(default)]
    pub cache: CacheJson,
    /// Watch options.
    #[serde(default)]
    pub watch: WatchJson,
    /// Daemon options.
    #[serde(default)]
    pub daemon: DaemonJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, TargetJson>>,
    /// Named reusable toolchain and runtime environments.
    pub environments: Option<IndexMap<String, EnvironmentJson>>,
    /// Named profiles for semantic configuration.
    pub profiles: Option<IndexMap<String, ProfileConfigJson>>,
    /// Named modes for emitted output policy.
    pub modes: Option<IndexMap<String, ModeJson>>,
    /// Default target for the package.
    pub default_target: Option<String>,
}

impl DestackJson {
    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        let specifiers = match &self.extends {
            Some(ExtendsFieldJson::Single(specifier)) => {
                vec![specifier.as_str()]
            }
            Some(ExtendsFieldJson::Multiple(specifiers)) => {
                specifiers.iter().map(String::as_str).collect()
            }
            None => Vec::new(),
        };

        specifiers.into_iter()
    }
}

/// Value for the "extends" field of a Destack config.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ExtendsFieldJson {
    /// Extend a single config path.
    Single(String),
    /// Extend multiple config paths.
    Multiple(Vec<String>),
}
