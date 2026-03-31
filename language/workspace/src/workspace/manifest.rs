use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileContent, FileId, Uri};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{config::Destack, config::TaskOptions};

/// Package manifest from `package.json`.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    /// The id of the `package.json` file.
    pub file_id: FileId,
    /// The uri of the `package.json` file.
    pub uri: Uri,
    /// The physical path to the `package.json` file.
    pub path: PathBuf,
    /// The resolved package name.
    pub name: String,
    /// The resolved package version.
    pub version: String,
    /// The realpath to the manifest.
    pub realpath: PathBuf,
    /// The manifest directory.
    pub directory: PathBuf,
    /// The raw manifest content before destack overrides.
    pub raw_content: PackageJson,
    /// The effective manifest content.
    pub content: PackageJson,
}

impl PackageManifest {
    /// Parse one package manifest from one json file.
    pub fn parse(file: &Arc<File>, realpath: PathBuf) -> Result<Self, serde_json::Error> {
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        let package_json: PackageJson = serde_json::from_value(value.clone())?;
        let path = file
            .uri
            .to_path_buf()
            .expect("package.json file must have a valid path");
        let directory = path
            .parent()
            .expect("package.json must have a parent directory")
            .to_path_buf();

        Ok(Self {
            file_id: file.id,
            uri: file.uri.clone(),
            path,
            name: package_json.name.clone().unwrap_or_default(),
            version: package_json.version.clone().unwrap_or_default(),
            realpath,
            directory,
            raw_content: package_json.clone(),
            content: package_json,
        })
    }

    /// Build one synthetic manifest from one destack config.
    pub fn from_destack(config: &Destack, realpath: PathBuf) -> Self {
        let content = PackageJson::from_destack(config);

        Self {
            file_id: config.file_id,
            uri: Uri::from_path(&config.path),
            path: config.path.clone(),
            name: content.name.clone().unwrap_or_default(),
            version: content.version.clone().unwrap_or_default(),
            realpath,
            directory: config.directory.clone(),
            raw_content: content.clone(),
            content,
        }
    }

    /// Refresh effective manifest fields from destack overrides.
    pub fn refresh_from_destack(&mut self, config: Option<&Destack>) {
        let mut content = self.raw_content.clone();

        if let Some(config) = config {
            content.apply_destack_overrides(config);
        }

        self.name = content.name.clone().unwrap_or_default();
        self.version = content.version.clone().unwrap_or_default();
        self.content = content;
    }
}

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
    /// Build manifest compatibility content from one destack config.
    pub fn from_destack(config: &Destack) -> Self {
        let scripts = Self::scripts_from_tasks(&config.options.tasks);
        let workspaces = Self::workspaces_from_destack(config);

        Self {
            name: config.options.name.clone(),
            version: config.options.version.clone(),
            is_private: config.options.is_private,
            description: config.options.description.clone(),
            license: config.options.license.clone(),
            repository: config.options.repository.clone(),
            homepage: config.options.homepage.clone(),
            keywords: Some(config.options.keywords.clone()),
            package_manager: config.options.package_manager.clone(),
            module_type: config.options.module_type.clone(),
            engines: Some(config.options.engines.clone()),
            exports: config.options.exports.clone(),
            imports: Some(config.options.imports.clone().into_iter().collect()),
            dependencies: Some(config.options.dependencies.clone()),
            dev_dependencies: Some(config.options.dev_dependencies.clone()),
            peer_dependencies: Some(config.options.peer_dependencies.clone()),
            optional_dependencies: Some(config.options.optional_dependencies.clone()),
            scripts,
            workspaces,
            ..Self::default()
        }
    }

    /// Apply destack manifest overrides in place.
    pub fn apply_destack_overrides(&mut self, config: &Destack) {
        let scripts = Self::scripts_from_tasks(&config.options.tasks);

        if let Some(name) = config.options.name.clone() {
            self.name = Some(name);
        }
        if let Some(version) = config.options.version.clone() {
            self.version = Some(version);
        }
        if let Some(is_private) = config.options.is_private {
            self.is_private = Some(is_private);
        }
        if let Some(description) = config.options.description.clone() {
            self.description = Some(description);
        }
        if let Some(license) = config.options.license.clone() {
            self.license = Some(license);
        }
        if let Some(repository) = config.options.repository.clone() {
            self.repository = Some(repository);
        }
        if let Some(homepage) = config.options.homepage.clone() {
            self.homepage = Some(homepage);
        }
        if !config.options.keywords.is_empty() {
            self.keywords = Some(config.options.keywords.clone());
        }
        if let Some(package_manager) = config.options.package_manager.clone() {
            self.package_manager = Some(package_manager);
        }
        if let Some(module_type) = config.options.module_type.clone() {
            self.module_type = Some(module_type);
        }
        if !config.options.engines.is_empty() {
            self.engines = Some(config.options.engines.clone());
        }
        if let Some(exports) = config.options.exports.clone() {
            self.exports = Some(exports);
        }
        if !config.options.imports.is_empty() {
            self.imports = Some(config.options.imports.clone().into_iter().collect());
        }
        if !config.options.dependencies.is_empty() {
            self.dependencies = Some(config.options.dependencies.clone());
        }
        if !config.options.dev_dependencies.is_empty() {
            self.dev_dependencies = Some(config.options.dev_dependencies.clone());
        }
        if !config.options.peer_dependencies.is_empty() {
            self.peer_dependencies = Some(config.options.peer_dependencies.clone());
        }
        if !config.options.optional_dependencies.is_empty() {
            self.optional_dependencies = Some(config.options.optional_dependencies.clone());
        }
        if let Some(scripts) = scripts {
            self.scripts = Some(scripts);
        }
        if let Some(workspaces) = Self::workspaces_from_destack(config) {
            self.workspaces = Some(workspaces);
        }
    }

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

    /// Project package scripts from destack tasks.
    fn scripts_from_tasks(
        tasks: &IndexMap<String, TaskOptions>,
    ) -> Option<IndexMap<String, String>> {
        let scripts: IndexMap<String, String> = tasks
            .iter()
            .filter_map(|(name, task)| {
                task.command
                    .as_ref()
                    .map(|command| (name.clone(), command.clone()))
            })
            .collect();

        if scripts.is_empty() {
            return None;
        }

        Some(scripts)
    }

    /// Project workspace membership to package manager compatibility.
    fn workspaces_from_destack(config: &Destack) -> Option<WorkspacesField> {
        if config.options.workspace.members.is_empty() {
            return None;
        }

        Some(WorkspacesField::Patterns(
            config.options.workspace.members.clone(),
        ))
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
