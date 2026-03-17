use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{File, FileContent, FileId, PackageId, PackageVersion, Uri};
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::{Destack, Target, TargetId, TaskOptions, TsConfigId};

/// Kind of package based on how it was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    /// Physical package on disk with package.json.
    Physical,
    /// Synthetic package for loose files (no package.json found).
    Synthetic,
    /// Ephemeral package for virtual content (REPL, eval, root module).
    Ephemeral,
    /// Builtin package for language primitives (operators, reflection, etc.).
    Builtin,
}

/// A Package is a bundle of modules.
#[derive(Debug, Clone)]
pub struct Package {
    /// The id of the Package.
    pub id: PackageId,
    /// The incremental package version.
    pub package_version: PackageVersion,
    /// The kind of the Package.
    pub kind: PackageKind,
    /// The URI of the package.
    pub uri: Uri,
    /// The path to the package directory (None for ephemeral packages).
    pub path: Option<PathBuf>,
    /// The name of the package.
    pub name: Option<String>,
    /// The version of the package.
    pub version: Option<String>,

    /// The package.json config (None for synthetic/ephemeral packages).
    pub manifest: Option<PackageManifest>,
    /// The destack.json config (1:1 with package, None if not specified).
    pub config: Option<Destack>,
    /// The root tsconfig of the package (in TsConfigRegistry, supports nesting).
    pub tsconfig: Option<TsConfigId>,
    /// Build targets for this package (usually from `destack.json`.targets).
    pub targets: IndexMap<TargetId, Target>,
}

impl Package {
    /// Get a target by name.
    pub fn target(&self, target: &TargetId) -> Option<&Target> {
        self.targets.get(target)
    }

    /// Get the default target (first one, if any).
    pub fn default_target(&self) -> Option<&Target> {
        self.targets.values().find(|target| !target.synthetic)
    }
}

/// Package manifest from `package.json`.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    /// The id of the `package.json` file.
    pub file_id: FileId,
    /// The URI of the `package.json` file.
    pub uri: Uri,
    /// The path to the `package.json` file.
    pub path: PathBuf,
    /// The name of the package.
    pub name: String,
    /// The version of the package.
    pub version: String,
    /// The realpath to the `package.json` file.
    pub realpath: PathBuf,
    /// The directory of the `package.json` file.
    pub directory: PathBuf,
    /// The raw package manifest content before Destack overrides.
    pub raw_content: PackageJson,
    /// The content of the `package.json` file.
    pub content: PackageJson,
}

impl PackageManifest {
    /// Parse a package.json file from a File with JSON content.
    pub fn parse(file: &Arc<File>, realpath: PathBuf) -> Result<Self, serde_json::Error> {
        // extract the JSON value from file content
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        // parse package.json from the JSON value
        let package_json: PackageJson = serde_json::from_value(value.clone())?;

        // extract path from file URI
        let path = file
            .uri
            .to_path_buf()
            .expect("package.json file must have a valid path");
        let directory = path
            .parent()
            .expect("package.json must have a parent directory")
            .to_path_buf();

        let package = Self {
            file_id: file.id,
            uri: file.uri.clone(),
            path,
            name: package_json.name.clone().unwrap_or_default(),
            version: package_json.version.clone().unwrap_or_default(),
            realpath,
            directory,
            raw_content: package_json.clone(),
            content: package_json,
        };
        Ok(package)
    }

    /// Build a synthetic manifest from one Destack config.
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

    /// Refresh effective manifest fields from raw package.json content and Destack overrides.
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

/// Package JSON (from `package.json`).
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    /// Name of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#name>
    pub name: Option<String>,

    /// Version of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#version>
    pub version: Option<String>,

    /// Whether the package is private.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#private>
    #[serde(rename = "private")]
    pub is_private: Option<bool>,

    /// Package description.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#description>
    pub description: Option<String>,

    /// Package license identifier.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#license>
    pub license: Option<String>,

    /// Package repository metadata.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#repository>
    pub repository: Option<Value>,

    /// Package homepage.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#homepage>
    pub homepage: Option<String>,

    /// Package keywords.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#keywords>
    pub keywords: Option<Vec<String>>,

    /// Preferred package manager string.
    /// <https://nodejs.org/api/packages.html#packagemanager>
    pub package_manager: Option<String>,

    /// Module type: "module" (ESM) or "commonjs" (CJS).
    /// <https://nodejs.org/api/packages.html#type>
    #[serde(rename = "type")]
    pub module_type: Option<String>,

    /// Supported runtime engines.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#engines>
    pub engines: Option<IndexMap<String, String>>,

    /// The "main" entry point.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#main>
    pub main: Option<String>,

    /// The "module" entry point for ECMAScript bundles.
    pub module: Option<String>,

    /// The "types" entry point. TypeScript types entry point of the package.
    pub types: Option<String>,

    /// The "browser" mapping. Browser-specific overrides.
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub browser: Option<Value>,

    /// The "exports" mapping. ECMAScript module exports.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#exports>
    pub exports: Option<Value>,

    /// The "imports" mapping. Node module imports.
    /// <https://nodejs.org/api/packages.html#imports>
    pub imports: Option<Map<String, Value>>,

    /// Runtime dependencies.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#dependencies>
    pub dependencies: Option<IndexMap<String, String>>,

    /// Development dependencies.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#devdependencies>
    pub dev_dependencies: Option<IndexMap<String, String>>,

    /// Peer dependencies.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#peerdependencies>
    pub peer_dependencies: Option<IndexMap<String, String>>,

    /// Optional dependencies.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#optionaldependencies>
    pub optional_dependencies: Option<IndexMap<String, String>>,

    /// The "scripts" map for CLI tasks.
    /// <https://docs.npmjs.com/cli/v11/using-npm/scripts>
    pub scripts: Option<IndexMap<String, String>>,

    /// The "bin" entry point field.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#bin>
    pub bin: Option<Value>,

    /// The "workspaces" field for npm/yarn/pnpm workspaces.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#workspaces>
    pub workspaces: Option<WorkspacesField>,
}

impl PackageJson {
    /// Build package manifest compatibility content from one Destack config.
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

    /// Apply Destack manifest overrides in place.
    pub fn apply_destack_overrides(&mut self, config: &Destack) {
        let scripts = Self::scripts_from_tasks(&config.options.tasks);

        // package identity
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

        // module interface
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

        // dependency intent
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

        // task compatibility
        if let Some(scripts) = scripts {
            self.scripts = Some(scripts);
        }

        // workspace compatibility
        if let Some(workspaces) = Self::workspaces_from_destack(config) {
            self.workspaces = Some(workspaces);
        }
    }

    /// Project package compatibility scripts from Destack tasks.
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

    /// Collect ordered package entry targets from package.json fields.
    pub fn entry_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        let mut seen = HashSet::new();

        // collect direct string entry fields
        Self::collect_string_target(self.main.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.module.as_deref(), &mut targets, &mut seen);
        Self::collect_string_target(self.types.as_deref(), &mut targets, &mut seen);

        // collect nested bin and exports entry targets
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

    /// Collect nested string targets from one JSON value.
    fn collect_target_values(value: &Value, targets: &mut Vec<String>, seen: &mut HashSet<String>) {
        match value {
            // collect one direct target
            Value::String(target) => {
                Self::collect_string_target(Some(target.as_str()), targets, seen);
            }
            // recursively collect array values
            Value::Array(values) => {
                for item in values {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            // recursively collect object values
            Value::Object(entries) => {
                for item in entries.values() {
                    Self::collect_target_values(item, targets, seen);
                }
            }
            // skip unsupported scalar targets
            _ => {}
        }
    }
}

/// The "workspaces" field in package.json.
/// Can be an array of glob patterns or an object with packages/nohoist.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum WorkspacesField {
    /// Array of workspace glob patterns.
    Patterns(Vec<String>),
    /// Object with packages array and optional nohoist.
    Object {
        /// Workspace package glob patterns.
        packages: Option<Vec<String>>,
        /// Packages to not hoist (yarn).
        nohoist: Option<Vec<String>>,
    },
}

impl WorkspacesField {
    /// Get the workspace patterns.
    pub fn patterns(&self) -> &[String] {
        match self {
            WorkspacesField::Patterns(patterns) => patterns,
            WorkspacesField::Object { packages, .. } => {
                packages.as_ref().map_or(&[], |p| p.as_slice())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::{PackageJson, WorkspacesField};
    use serde_json::json;

    use destack_source::{File, FileId, FileType, Uri};

    use crate::Destack;

    /// Collect package entry targets in stable field order.
    #[test]
    fn test_collect_package_json_entry_targets() {
        let package_json: PackageJson = serde_json::from_value(json!({
            "name": "test",
            "main": "./dist/index.js",
            "module": "./dist/index.mjs",
            "types": "./dist/index.d.ts",
            "bin": {
                "test": "./bin/test.js"
            },
            "exports": {
                ".": {
                    "import": "./esm/index.js",
                    "require": "./cjs/index.cjs",
                    "types": "./types/index.d.ts"
                }
            }
        }))
        .expect("failed to parse package json");

        assert_eq!(
            package_json.entry_targets(),
            vec![
                "./dist/index.js",
                "./dist/index.mjs",
                "./dist/index.d.ts",
                "./bin/test.js",
                "./esm/index.js",
                "./cjs/index.cjs",
                "./types/index.d.ts"
            ]
        );
    }

    /// Skip duplicate and empty package entry targets.
    #[test]
    fn test_collect_package_json_entry_targets_skips_duplicates() {
        let package_json: PackageJson = serde_json::from_value(json!({
            "main": "./dist/index.js",
            "module": "./dist/index.js",
            "types": "",
            "exports": ["./dist/index.js", "./dist/other.js"]
        }))
        .expect("failed to parse package json");

        assert_eq!(
            package_json.entry_targets(),
            vec!["./dist/index.js", "./dist/other.js"]
        );
    }

    /// Project package compatibility fields from one Destack config.
    #[test]
    fn test_package_json_from_destack_projects_manifest_fields() {
        let path = PathBuf::from("/tmp/app/destack.json");
        let uri = Uri::from_string("file:///tmp/app/destack.json");
        let file = Arc::new(
            File::from_text_as_json(
                FileId::new(1),
                "destack.json".to_string(),
                uri,
                Some(path),
                FileType::Json,
                json!({
                    "name": "@destack/app",
                    "version": "0.1.0",
                    "private": true,
                    "description": "demo app",
                    "license": "MIT",
                    "repository": {
                        "type": "git",
                        "url": "https://example.com/repo.git"
                    },
                    "homepage": "https://example.com",
                    "keywords": ["destack", "demo"],
                    "packageManager": "pnpm@10.0.0",
                    "type": "module",
                    "engines": {
                        "node": ">=22"
                    },
                    "exports": {
                        ".": "./dist/index.js"
                    },
                    "imports": {
                        "#app": "./src/index.ts"
                    },
                    "dependencies": {
                        "react": "^19.0.0"
                    },
                    "devDependencies": {
                        "typescript": "^5.9.0"
                    },
                    "peerDependencies": {
                        "react-dom": "^19.0.0"
                    },
                    "optionalDependencies": {
                        "fsevents": "^2.3.0"
                    },
                    "tasks": {
                        "dev": "destack dev",
                        "build": {
                            "command": "destack build"
                        }
                    },
                    "workspace": {
                        "members": ["apps/*", "packages/*"],
                        "groups": {
                            "product": ["apps/web", "apps/api"]
                        }
                    }
                })
                .to_string(),
            )
            .expect("destack file should parse as json"),
        );
        let config = Destack::parse(&file).expect("destack config should parse");

        let package_json = PackageJson::from_destack(&config);

        assert_eq!(package_json.name.as_deref(), Some("@destack/app"));
        assert_eq!(package_json.version.as_deref(), Some("0.1.0"));
        assert_eq!(package_json.is_private, Some(true));
        assert_eq!(package_json.description.as_deref(), Some("demo app"));
        assert_eq!(package_json.license.as_deref(), Some("MIT"));
        assert_eq!(
            package_json.homepage.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            package_json.keywords.as_ref().cloned(),
            Some(vec!["destack".to_string(), "demo".to_string()])
        );
        assert_eq!(package_json.package_manager.as_deref(), Some("pnpm@10.0.0"));
        assert_eq!(package_json.module_type.as_deref(), Some("module"));
        assert_eq!(
            package_json
                .engines
                .as_ref()
                .and_then(|engines| engines.get("node"))
                .map(String::as_str),
            Some(">=22")
        );
        assert_eq!(
            package_json
                .dependencies
                .as_ref()
                .and_then(|dependencies| dependencies.get("react"))
                .map(String::as_str),
            Some("^19.0.0")
        );
        assert_eq!(
            package_json
                .dev_dependencies
                .as_ref()
                .and_then(|dependencies| dependencies.get("typescript"))
                .map(String::as_str),
            Some("^5.9.0")
        );
        assert_eq!(
            package_json
                .peer_dependencies
                .as_ref()
                .and_then(|dependencies| dependencies.get("react-dom"))
                .map(String::as_str),
            Some("^19.0.0")
        );
        assert_eq!(
            package_json
                .optional_dependencies
                .as_ref()
                .and_then(|dependencies| dependencies.get("fsevents"))
                .map(String::as_str),
            Some("^2.3.0")
        );
        assert_eq!(
            package_json
                .scripts
                .as_ref()
                .and_then(|scripts| scripts.get("dev"))
                .map(String::as_str),
            Some("destack dev")
        );
        assert_eq!(
            package_json
                .scripts
                .as_ref()
                .and_then(|scripts| scripts.get("build"))
                .map(String::as_str),
            Some("destack build")
        );
        assert_eq!(
            package_json
                .workspaces
                .as_ref()
                .map(WorkspacesField::patterns)
                .map(<[String]>::len),
            Some(2)
        );
    }
}

/// Registry of Packages. THREAD-SAFE.
#[derive(Debug)]
pub struct PackageRegistry {
    /// The packages by id.
    packages_by_id: DashMap<PackageId, Arc<RwLock<Package>>>,
    /// URI-based index for looking up packages by their URI.
    packages_by_uri: DashMap<Uri, PackageId>,
    /// Path-based index for looking up packages by their directory path.
    packages_by_path: DashMap<PathBuf, PackageId>,
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageRegistry {
    /// Create a new PackageRegistry.
    pub fn new() -> Self {
        Self {
            packages_by_id: DashMap::new(),
            packages_by_uri: DashMap::new(),
            packages_by_path: DashMap::new(),
        }
    }

    /// Insert a package into the registry.
    pub fn insert(&self, package: Package) {
        let uri = package.uri.clone();
        let path = package.path.clone();
        let id = package.id;
        self.packages_by_id
            .insert(id, Arc::new(RwLock::new(package)));
        self.packages_by_uri.insert(uri, id);
        if let Some(path) = path {
            self.packages_by_path.insert(path, id);
        }
    }

    /// Check if a package exists by id.
    pub fn contains(&self, id: PackageId) -> bool {
        self.packages_by_id.contains_key(&id)
    }

    /// Get a package by package id.
    ///
    /// # Panics
    /// Panics if the package is not found.
    #[inline]
    pub fn get(&self, id: PackageId) -> Arc<RwLock<Package>> {
        self.packages_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("package not found for id: {id:?}"))
            .clone()
    }

    /// Get a package by package id when present.
    pub fn get_maybe(&self, id: PackageId) -> Option<Arc<RwLock<Package>>> {
        self.packages_by_id
            .get(&id)
            .map(|entry| entry.value().clone())
    }

    /// Get the current package version.
    ///
    /// # Panics
    /// Panics if the package is not found.
    pub fn version(&self, id: PackageId) -> PackageVersion {
        let package = self.get(id);
        let package = package.read();
        package.package_version
    }

    /// Bump the package version and return the new value.
    ///
    /// # Panics
    /// Panics if the package is not found.
    pub fn bump_version(&self, id: PackageId) -> PackageVersion {
        let package = self.get(id);
        let mut package = package.write();

        // bump package version
        let next_version = package.package_version.next();
        package.package_version = next_version;

        next_version
    }

    /// Get a package id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<PackageId> {
        self.packages_by_uri.get(uri).map(|r| *r.value())
    }

    /// Get a package by its URI.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<RwLock<Package>>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a package exists with the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.packages_by_uri.contains_key(uri)
    }

    /// Get a package id by its directory path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<PackageId> {
        self.packages_by_path.get(path).map(|r| *r.value())
    }

    /// Get a package by its directory path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<RwLock<Package>>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a package exists at the given directory path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.packages_by_path.contains_key(path)
    }

    /// Iterate over the packages in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Package>>> {
        let snapshot: Vec<_> = self
            .packages_by_id
            .iter()
            .map(|r| r.value().clone())
            .collect();
        snapshot.into_iter()
    }

    /// Get the number of packages in the registry.
    pub fn len(&self) -> usize {
        self.packages_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.packages_by_id.is_empty()
    }
}
