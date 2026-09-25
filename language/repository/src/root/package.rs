use std::path::{Path, PathBuf};
use std::sync::Arc;

use im::OrdMap;
use indexmap::IndexMap;
use tspp_source::{FileId, PackageId, TargetId, Uri};

use crate::config::{
    ConditionGate, ConditionSet, Dependency, ManifestFile, Target, Topology, Vendor,
};

/// The ownership kind for a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    /// Embedded package shipped with the compiler.
    Embedded,
    /// Declared package rooted by authored workspace config.
    Declared,
    /// Package captured because another package depends on it.
    Dependency,
    /// Implicit package rooted by one loose-file directory.
    Implicit,
}

impl PackageKind {
    /// Return whether this package contains authored workspace source.
    pub const fn is_authored(self) -> bool {
        matches!(self, Self::Declared | Self::Implicit)
    }
}

/// One package of modules.
#[derive(Debug, Clone)]
pub struct Package {
    /// The package id.
    pub id: PackageId,
    /// The ownership kind.
    pub kind: PackageKind,
    /// Whether this is the canonical builtin language package.
    pub is_builtin: bool,
    /// The package uri.
    pub uri: Uri,
    /// The package directory when filesystem backed.
    pub path: Option<PathBuf>,
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// Package dependencies enabled unconditionally.
    pub dependencies: IndexMap<String, Dependency>,
    /// Package dependencies enabled by source graph conditions.
    pub conditional_dependencies: Vec<PackageDependencies>,
    /// Vendored dependency resolution options.
    pub vendor: Vendor,
    /// Public package exports.
    pub exports: IndexMap<String, PackageExport>,
    /// Package topology definition.
    pub topology: Topology,
    /// The package targets.
    pub targets: IndexMap<TargetId, Target>,
    /// The effective package configuration when present.
    pub configuration: Option<Arc<ManifestFile>>,
}

impl Package {
    /// Get one target by id.
    pub fn target(&self, target: &TargetId) -> Option<&Target> {
        self.targets.get(target)
    }

    /// Get one package export by key.
    pub fn export(&self, key: &str) -> Option<&PackageExport> {
        self.exports.get(key)
    }

    /// Return dependencies enabled by the active source graph conditions.
    pub fn dependencies_for_conditions(
        &self,
        conditions: &ConditionSet,
    ) -> IndexMap<String, Dependency> {
        let mut dependencies = self.dependencies.clone();

        for conditional in &self.conditional_dependencies {
            if !conditional.matches(conditions) {
                continue;
            }

            for (name, dependency) in &conditional.dependencies {
                dependencies.insert(name.clone(), dependency.clone());
            }
        }

        dependencies
    }
}

/// Dependencies enabled by one resolved condition gate.
#[derive(Debug, Clone, Default)]
pub struct PackageDependencies {
    /// Condition gate enabling these dependencies.
    pub when: ConditionGate,
    /// Dependency declarations enabled when the gate matches.
    pub dependencies: IndexMap<String, Dependency>,
}

impl PackageDependencies {
    /// Return whether these dependencies are active for one condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        self.when.matches(conditions)
    }
}

/// Public package export after condition references are resolved.
#[derive(Debug, Clone)]
pub struct PackageExport {
    /// The candidate paths in declaration order.
    pub cases: Vec<ExportCase>,
}

impl PackageExport {
    /// Return the first path whose condition gate matches one condition set.
    pub fn select(&self, conditions: &ConditionSet) -> Option<&str> {
        self.cases
            .iter()
            .find(|case| {
                case.when
                    .as_ref()
                    .is_none_or(|gate| gate.matches(conditions))
            })
            .map(|case| case.path.as_str())
    }

    /// Return the path of an export with one unconditional case.
    pub fn path(&self) -> Option<&str> {
        match self.cases.as_slice() {
            [ExportCase { when: None, path }] => Some(path),
            _ => None,
        }
    }
}

/// One candidate path of a package export.
#[derive(Debug, Clone)]
pub struct ExportCase {
    /// Condition gate selecting this path, none for an unconditional path.
    pub when: Option<ConditionGate>,
    /// Package relative path.
    pub path: String,
}

/// Revision-local package lookup data.
#[derive(Debug, Clone)]
pub(crate) struct PackageIndex {
    /// Packages keyed by package id.
    packages: OrdMap<PackageId, Arc<Package>>,
    /// Package roots ordered from most specific to least specific.
    roots: Vec<(PathBuf, PackageId)>,
    /// Files contributing to workspace and package configuration, ordered by identity.
    configuration_files: Vec<FileId>,
}

impl PackageIndex {
    /// Build one package index from resolved packages.
    pub(crate) fn new(
        packages: OrdMap<PackageId, Arc<Package>>,
        workspace: Option<&ManifestFile>,
    ) -> Self {
        // index every referenced workspace and package configuration file
        let mut configuration_files = workspace
            .into_iter()
            .chain(
                packages
                    .values()
                    .filter_map(|package| package.configuration.as_deref()),
            )
            .flat_map(|configuration| configuration.file_ids.iter().copied())
            .collect::<Vec<_>>();
        configuration_files.sort_unstable();
        configuration_files.dedup();

        // order package roots from most specific to least specific
        let mut roots = packages
            .values()
            .filter_map(|package| package.path.as_ref().map(|path| (path.clone(), package.id)))
            .collect::<Vec<_>>();

        roots.sort_by(|left, right| {
            right
                .0
                .as_os_str()
                .len()
                .cmp(&left.0.as_os_str().len())
                .then_with(|| left.0.cmp(&right.0))
        });

        Self {
            packages,
            roots,
            configuration_files,
        }
    }

    /// Return whether a file contributes to workspace or package configuration.
    pub(crate) fn is_configuration_file(&self, file: FileId) -> bool {
        self.configuration_files.binary_search(&file).is_ok()
    }

    /// Return one package by id.
    pub(crate) fn package(&self, package_id: PackageId) -> Option<Arc<Package>> {
        self.packages.get(&package_id).cloned()
    }

    /// Return one package by declared package name.
    pub(crate) fn package_by_name(&self, name: &str) -> Option<Arc<Package>> {
        self.packages
            .values()
            .find(|package| package.name.as_deref() == Some(name))
            .cloned()
    }

    /// Return one package by package root path.
    pub(crate) fn package_by_path(&self, path: &Path) -> Option<Arc<Package>> {
        self.packages
            .values()
            .find(|package| package.path.as_deref() == Some(path))
            .cloned()
    }

    /// Return all package ids.
    pub(crate) fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.packages.keys().copied()
    }

    /// Return package roots ordered from most specific to least specific.
    pub(crate) fn roots(&self) -> &[(PathBuf, PackageId)] {
        &self.roots
    }

    /// Return the nearest package for one path.
    pub(crate) fn nearest_package(&self, path: &Path) -> Option<Arc<Package>> {
        for (root, package_id) in &self.roots {
            if path.starts_with(root) {
                return self.package(*package_id);
            }
        }

        None
    }
}
