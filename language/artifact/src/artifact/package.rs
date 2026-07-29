use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_serde::Reflect;
use destack_source::{ModuleId, PackageId, ProfileId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Active package routes and import specifiers for one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct PackageGraph {
    /// The profile this graph belongs to.
    pub profile: ProfileId,
    /// Active package nodes keyed by package id.
    nodes: IndexMap<PackageId, PackageNode>,
    /// Canonical same-package import paths ordered by module.
    module_paths: Vec<ModuleImportPath>,
    /// Public import specifiers ordered by source package, target module, and text.
    package_specifiers: Vec<PackageImportSpecifier>,
}

impl PackageGraph {
    /// Build a package graph from active packages and exact import specifiers.
    pub fn new(
        profile: ProfileId,
        nodes: IndexMap<PackageId, PackageNode>,
        module_paths: impl IntoIterator<Item = (ModuleId, PathBuf)>,
        package_specifiers: impl IntoIterator<Item = (PackageId, ModuleId, String)>,
    ) -> Self {
        let mut module_paths = module_paths
            .into_iter()
            .map(|(module, path)| ModuleImportPath { module, path })
            .collect::<Vec<_>>();
        module_paths.sort();
        module_paths.dedup();

        let mut package_specifiers = package_specifiers
            .into_iter()
            .map(|(source, target, specifier)| PackageImportSpecifier {
                source,
                target,
                specifier,
            })
            .collect::<Vec<_>>();
        package_specifiers.sort();
        package_specifiers.dedup();

        Self {
            profile,
            nodes,
            module_paths,
            package_specifiers,
        }
    }

    /// Return one active package.
    pub fn package(&self, package: PackageId) -> Option<&PackageNode> {
        self.nodes.get(&package)
    }

    /// Return the canonical same-package import path for one module.
    pub fn module_path(&self, module: ModuleId) -> Option<&Path> {
        let index = self
            .module_paths
            .binary_search_by_key(&module, |entry| entry.module)
            .ok()?;

        Some(self.module_paths[index].path.as_path())
    }

    /// Iterate public specifiers from one source package to one target module.
    pub fn package_specifiers(
        &self,
        source: PackageId,
        target: ModuleId,
    ) -> impl Iterator<Item = &str> {
        let key = (source, target);
        let start = self
            .package_specifiers
            .partition_point(|entry| (entry.source, entry.target) < key);
        let end = start
            + self.package_specifiers[start..]
                .partition_point(|entry| (entry.source, entry.target) == key);

        self.package_specifiers[start..end]
            .iter()
            .map(|entry| entry.specifier.as_str())
    }
}

/// One package in an active package graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PackageNode {
    /// Package root path when filesystem backed.
    pub root: Option<PathBuf>,
    /// Active direct dependencies keyed by package specifier.
    pub dependencies: IndexMap<String, PackageDependency>,
    /// Active public exports.
    pub exports: PackageExports,
}

impl PackageNode {
    /// Return one active direct dependency by package specifier.
    pub fn dependency(&self, package: &str) -> Option<PackageDependency> {
        self.dependencies.get(package).copied()
    }
}

/// Resolution of one declared package dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PackageDependency {
    /// The declared dependency package is unavailable.
    Unavailable,
    /// The declared dependency resolved to an active package.
    Resolved(PackageId),
}

/// Active package exports indexed for module import resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct PackageExports {
    /// Exact exports keyed by export specifier.
    pub exact: IndexMap<String, ExportTarget>,
    /// Pattern exports sorted from most specific to least specific.
    pub patterns: Vec<ExportPattern>,
}

impl PackageExports {
    /// Return the active export matching one export key.
    pub fn get<'a>(&'a self, key: &'a str) -> Option<ResolvedExport<'a>> {
        // prefer exact exports
        if let Some(target) = self.exact.get(key) {
            return Some(ResolvedExport {
                target,
                path: Cow::Borrowed(target.path.as_str()),
            });
        }

        // scan active patterns in specificity order
        for pattern in &self.patterns {
            let Some(replacement) = pattern.replacement(key) else {
                continue;
            };
            let path = pattern.target.path.replace('*', replacement);

            return Some(ResolvedExport {
                target: &pattern.target,
                path: Cow::Owned(path),
            });
        }

        None
    }
}

/// One canonical same-package import path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
struct ModuleImportPath {
    /// The module selected by this path.
    module: ModuleId,
    /// The shortest exact path accepted by module resolution.
    path: PathBuf,
}

/// One public package import specifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
struct PackageImportSpecifier {
    /// The package containing the importing module.
    source: PackageId,
    /// The module selected by the specifier.
    target: ModuleId,
    /// The authored module specifier.
    specifier: String,
}

/// Active package export target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ExportTarget {
    /// Package relative export path.
    pub path: String,
    /// Whether the export can be imported as a source module.
    pub is_module: bool,
}

/// Active pattern export target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ExportPattern {
    /// Export key prefix before `*`.
    pub prefix: String,
    /// Export key suffix after `*`.
    pub suffix: String,
    /// Pattern export target.
    pub target: ExportTarget,
}

impl ExportPattern {
    /// Return the replacement matched by this pattern.
    fn replacement<'a>(&self, key: &'a str) -> Option<&'a str> {
        // require matching edges
        if !key.starts_with(&self.prefix) || !key.ends_with(&self.suffix) {
            return None;
        }

        // slice the wildcard body
        let start = self.prefix.len();
        let end = key.len().checked_sub(self.suffix.len())?;

        // reject overlapping pattern edges
        if start > end {
            return None;
        }

        Some(&key[start..end])
    }
}

/// Resolved active package export.
#[derive(Debug, Clone)]
pub struct ResolvedExport<'a> {
    /// Matched export target.
    pub target: &'a ExportTarget,
    /// Resolved package relative path.
    pub path: Cow<'a, str>,
}
