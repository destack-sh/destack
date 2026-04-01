use std::collections::HashSet;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::{Map, Value};

/// Package JSON content from `package.json`.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// Whether the package is private.
    #[serde(rename = "private")]
    pub is_private: Option<bool>,
    /// The package description.
    pub description: Option<String>,
    /// The package license identifier.
    pub license: Option<String>,
    /// The repository metadata.
    pub repository: Option<Value>,
    /// The homepage URL.
    pub homepage: Option<String>,
    /// The package keywords.
    pub keywords: Option<Vec<String>>,
    /// The preferred package manager string.
    pub package_manager: Option<String>,
    /// The module type field.
    #[serde(rename = "type")]
    pub module_type: Option<String>,
    /// Supported runtime engines.
    pub engines: Option<IndexMap<String, String>>,
    /// The `main` entry point.
    pub main: Option<String>,
    /// The `module` entry point.
    pub module: Option<String>,
    /// The `types` entry point.
    pub types: Option<String>,
    /// The browser mapping.
    pub browser: Option<Value>,
    /// The exports mapping.
    pub exports: Option<Value>,
    /// The imports mapping.
    pub imports: Option<Map<String, Value>>,
    /// Runtime dependencies.
    pub dependencies: Option<IndexMap<String, String>>,
    /// Development dependencies.
    pub dev_dependencies: Option<IndexMap<String, String>>,
    /// Peer dependencies.
    pub peer_dependencies: Option<IndexMap<String, String>>,
    /// Optional dependencies.
    pub optional_dependencies: Option<IndexMap<String, String>>,
    /// The scripts map.
    pub scripts: Option<IndexMap<String, String>>,
    /// The `bin` field.
    pub bin: Option<Value>,
    /// The workspace field.
    pub workspaces: Option<WorkspacesField>,
}

impl PackageJson {
    /// Collect package entry targets in stable field order.
    pub fn entry_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        let mut seen = HashSet::new();

        Self::collect_string_target(self.main.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.module.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.types.as_deref(), &mut targets, &mut seen);

        if let Some(bin) = self.bin.as_ref() {
            Self::collect_target_values(bin, &mut targets, &mut seen);
        }
        if let Some(exports) = self.exports.as_ref() {
            Self::collect_target_values(exports, &mut targets, &mut seen);
        }

        targets
    }

    /// Collect one optional string target.
    fn collect_string_target(
        value: Option<&str>,
        targets: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        let Some(value) = value else {
            return;
        };

        let target = value.trim();
        if target.is_empty() {
            return;
        }

        let target = target.to_string();
        if seen.insert(target.clone()) {
            targets.push(target);
        }
    }

    /// Collect nested string targets from one json value.
    fn collect_target_values(value: &Value, targets: &mut Vec<String>, seen: &mut HashSet<String>) {
        match value {
            Value::String(target) => {
                Self::collect_string_target(Some(target.as_str()), targets, seen);
            }
            Value::Array(values) => {
                for item in values {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            Value::Object(entries) => {
                for item in entries.values() {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            _ => {}
        }
    }
}

/// The `workspaces` field in `package.json`.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum WorkspacesField {
    /// Array of workspace glob patterns.
    Patterns(Vec<String>),
    /// Object form with packages and nohoist.
    Object {
        /// Workspace package patterns.
        packages: Option<Vec<String>>,
        /// Packages to not hoist.
        nohoist: Option<Vec<String>>,
    },
}

impl WorkspacesField {
    /// Return the workspace package patterns.
    pub fn patterns(&self) -> &[String] {
        match self {
            WorkspacesField::Patterns(patterns) => patterns,
            WorkspacesField::Object { packages, .. } => packages
                .as_ref()
                .map_or(&[], |patterns| patterns.as_slice()),
        }
    }
}
