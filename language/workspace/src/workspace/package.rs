use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId, TargetId, Uri};
use im::OrdMap;
use indexmap::IndexMap;

use crate::config::{DependencyMap, Target, Vendor};

/// The ownership kind for a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    /// Declared package rooted by authored workspace config.
    Declared,
    /// Implicit package rooted by one loose-file directory.
    Implicit,
}

/// One package of modules.
#[derive(Debug, Clone)]
pub struct Package {
    /// The package id.
    pub id: PackageId,
    /// The ownership kind.
    pub kind: PackageKind,
    /// The package uri.
    pub uri: Uri,
    /// The package directory when filesystem backed.
    pub path: Option<PathBuf>,
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// Package dependencies enabled for all modes.
    pub dependencies: DependencyMap,
    /// Package dependencies enabled by source graph mode.
    pub mode_dependencies: IndexMap<String, DependencyMap>,
    /// Vendored dependency resolution options.
    pub vendoring: Vendor,
    /// The `destack.json` file id when present.
    pub destack_file_id: Option<FileId>,
    /// The package targets.
    pub targets: IndexMap<TargetId, Target>,
}

impl Package {
    /// Get one target by id.
    pub fn target(&self, target: &TargetId) -> Option<&Target> {
        self.targets.get(target)
    }

    /// Return dependencies enabled by the active source graph modes.
    pub fn dependencies_for_modes(&self, modes: &[String]) -> DependencyMap {
        let mut dependencies = self.dependencies.clone();

        for mode in modes {
            let Some(mode_dependencies) = self.mode_dependencies.get(mode) else {
                continue;
            };

            for (name, dependency) in mode_dependencies {
                dependencies.insert(name.clone(), dependency.clone());
            }
        }

        dependencies
    }
}

/// Revision-local package lookup data.
#[derive(Debug, Clone)]
pub(crate) struct PackageIndex {
    /// Packages keyed by package id.
    packages: OrdMap<PackageId, Arc<Package>>,
    /// Package roots ordered from most specific to least specific.
    roots: Vec<(PathBuf, PackageId)>,
}

impl PackageIndex {
    /// Build one package index from resolved packages.
    pub(crate) fn new(packages: OrdMap<PackageId, Arc<Package>>) -> Self {
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

        Self { packages, roots }
    }

    /// Return the package count.
    pub(crate) fn len(&self) -> usize {
        self.packages.len()
    }

    /// Return one package by id.
    pub(crate) fn package(&self, package_id: PackageId) -> Option<Arc<Package>> {
        self.packages.get(&package_id).cloned()
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
