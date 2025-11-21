use core::fmt;
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;
use serde::de::Error;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::TsConfigJson;

/// Unique identifier for Packages.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackageId(pub u32);

impl PackageId {
    /// Wrap an id as a PackageId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Package is a bundle of modules.
#[derive(Debug, Clone)]
pub struct Package {
    /// The id of the Package.
    pub id: PackageId,
    /// The path to the package.
    pub path: PathBuf,
    /// The realpath to the package.
    pub realpath: PathBuf,
    /// The directory of the package.
    pub directory: PathBuf,
    /// The name of the package.
    pub name: Option<String>,
    /// The version of the package.
    pub version: Option<String>,
    /// The type of the package.
    pub ty: PackageType,

    /// The detailed options of the package.
    pub package_json: PackageJson,
    /// The tsconfig of the package.
    pub tsconfig_json: Option<TsConfigJson>,
}

/// The package type.
/// <https://nodejs.org/api/packages.html#type>
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    /// CommonJS package.
    CommonJs,
    /// Module package.
    Module,
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonJs => f.write_str("commonjs"),
            Self::Module => f.write_str("module"),
        }
    }
}

/// Package options (from `package.json`).
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    // nocheckin: remove path/realpath/directory from PackageJson (into Package)
    /// Path to `package.json` (including the `package.json` filename).
    #[serde(skip)]
    pub path: PathBuf,
    /// Realpath of `package.json` (including the `package.json` filename).
    #[serde(skip)]
    pub realpath: PathBuf,
    /// Directory of `package.json` (excluding the `package.json` filename).
    #[serde(skip)]
    pub directory: PathBuf,

    /// Name of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#name>
    pub name: Option<String>,
    /// Version of the package.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#version>
    pub version: Option<String>,
    /// Package type (Module or CommonJS).
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#type>
    #[serde(rename = "type")]
    pub ty: Option<PackageType>,
    /// The "main" entry point.
    /// <https://docs.npmjs.com/cli/v11/configuring-npm/package-json#main>
    pub main: Option<String>,
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
}

impl fmt::Debug for PackageJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageOptions")
            .field("path", &self.path)
            .field("realpath", &self.realpath)
            .field("name", &self.name)
            .field("type", &self.ty)
            .finish_non_exhaustive()
    }
}

impl PackageJson {
    /// Parse a package.json file from JSON bytes.
    pub fn parse(
        path: PathBuf,
        realpath: PathBuf,
        content: Vec<u8>,
    ) -> Result<Self, serde_json::Error> {
        // strip BOM - UTF-8 BOM is 3 bytes: 0xEF, 0xBB, 0xBF
        let json_bytes = if content.starts_with(b"\xEF\xBB\xBF") {
            &content[3..]
        } else {
            &content[..]
        };

        // check if content is empty(ish)
        if json_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
            return Err(serde_json::Error::custom("File is empty"));
        }

        // parse options
        let mut options: PackageJson = serde_json::from_slice(json_bytes)?;
        options.path = path;
        options.realpath = realpath;
        options.directory = options.path.parent().unwrap().to_path_buf();

        Ok(options)
    }
}

/// Graph of Packages (including their underlying Files). THREAD-SAFE.
#[derive(Debug)]
pub struct PackageRegistry {
    /// The packages by id.
    packages_by_id: Mutex<HashMap<PackageId, Arc<RwLock<Package>>>>,
    /// The next package id.
    next_package_id: AtomicU32,
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
            packages_by_id: Mutex::new(HashMap::new()),
            next_package_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next package id.
    pub fn next_id(&self) -> PackageId {
        let next_package_id = self.next_package_id.fetch_add(1, Ordering::Relaxed);
        PackageId::new(next_package_id)
    }

    /// Insert a package into the graph.
    pub fn insert(&self, package: Package) {
        let mut packages_by_id = self.packages_by_id.lock();
        packages_by_id.insert(package.id, Arc::new(RwLock::new(package)));
    }

    /// Get a package by package id.
    #[inline]
    pub fn get(&self, id: PackageId) -> Option<Arc<RwLock<Package>>> {
        let packages_by_id = self.packages_by_id.lock();
        packages_by_id.get(&id).cloned()
    }

    /// Iterate over the packages in the registry.
    pub fn iter(&self) -> impl Iterator<Item = Arc<RwLock<Package>>> {
        let packages_by_id = self.packages_by_id.lock();
        let snapshot: Vec<_> = packages_by_id.values().cloned().collect();
        snapshot.into_iter()
    }

    /// Get the number of packages in the registry.
    pub fn len(&self) -> usize {
        let packages_by_id = self.packages_by_id.lock();
        packages_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        let packages_by_id = self.packages_by_id.lock();
        packages_by_id.is_empty()
    }
}
