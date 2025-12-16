use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{File, FileContent, FileId, PackageId, Uri};
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{DsConfig, Target, TsConfigId};

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
    /// The dsconfig.json config (1:1 with package, None if not specified).
    pub dsconfig: Option<DsConfig>,
    /// The root tsconfig of the package (in TsConfigRegistry, supports nesting).
    pub tsconfig: Option<TsConfigId>,
    /// Build targets for this package (usually from `dsconfig.json`.targets).
    pub targets: IndexMap<String, Target>,
}

impl Package {
    /// Get a target by name.
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }

    /// Get the default target (first one, if any).
    pub fn default_target(&self) -> Option<&Target> {
        self.targets.values().next()
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
            content: package_json,
        };
        Ok(package)
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

    /// Module type: "module" (ESM) or "commonjs" (CJS).
    /// <https://nodejs.org/api/packages.html#type>
    #[serde(rename = "type")]
    pub module_type: Option<String>,

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

    /// The "workspaces" field for npm/yarn/pnpm workspaces.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#workspaces>
    pub workspaces: Option<WorkspacesField>,
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
