use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::PackageJson;

/// JS module type.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ModuleType {
    /// ESM module.
    Module,
    /// CommonJS module.
    CommonJs,
    /// JSON module.
    Json,
    /// Wasm module.
    Wasm,
    /// Addon module.
    Addon,
}

/// The final path resolution with optional `?query` and `#fragment`
pub struct Resolution {
    /// The final path.
    pub(crate) path: PathBuf,
    /// Query `?query`, contains `?` (like `?foo` in `foo.js?foo`).
    pub(crate) query: Option<String>,
    /// Fragment `#query`, contains `#` (like `#foo` in `foo.js#foo`).
    pub(crate) fragment: Option<String>,
    /// `package.json` of the given module.
    pub(crate) package_json: Option<Arc<PackageJson>>,
    /// Module type of this path.
    pub(crate) module_type: Option<ModuleType>,
}

impl Clone for Resolution {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            query: self.query.clone(),
            fragment: self.fragment.clone(),
            package_json: self.package_json.clone(),
            module_type: self.module_type,
        }
    }
}

impl fmt::Debug for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Resolution")
            .field("path", &self.path)
            .field("query", &self.query)
            .field("fragment", &self.fragment)
            .field("module_type", &self.module_type)
            .field(
                "package_json",
                &self.package_json.as_ref().map(|p| p.path()),
            )
            .finish()
    }
}

impl PartialEq for Resolution {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.query == other.query && self.fragment == other.fragment
    }
}
impl Eq for Resolution {}

impl Resolution {
    /// Returns the path without query and fragment
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path without query and fragment
    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }

    /// Returns the path query `?query`, contains the leading `?`
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Returns the path fragment `#fragment`, contains the leading `#`
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Returns serialized package_json
    pub fn package_json(&self) -> Option<&Arc<PackageJson>> {
        self.package_json.as_ref()
    }

    /// Returns the full path with query and fragment
    pub fn full_path(&self) -> PathBuf {
        let mut path = self.path.clone().into_os_string();
        if let Some(query) = &self.query {
            path.push(query);
        }
        if let Some(fragment) = &self.fragment {
            path.push(fragment);
        }
        PathBuf::from(path)
    }

    /// Returns the module type of this path.
    pub fn module_type(&self) -> Option<ModuleType> {
        self.module_type
    }
}
