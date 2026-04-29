use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, ModuleId, PackageId, TargetId};
use im::OrdMap;

use crate::{Module, Package, Target, WorkspaceError};

/// Kind of workspace based on how it was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorkspaceKind {
    /// Monorepo with multiple member projects.
    Monorepo,
    /// Single package workspace.
    #[default]
    SinglePackage,
}

/// A repository-derived workspace index for one revision.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The workspace config declaration file id when present.
    pub file_id: Option<FileId>,
    /// The workspace root directory.
    pub root: PathBuf,
    /// The workspace kind.
    pub kind: WorkspaceKind,

    /// The packages in this workspace.
    pub(crate) packages: OrdMap<PackageId, Arc<Package>>,
    /// The package roots ordered from most specific to least specific.
    pub(crate) package_roots: Vec<(PathBuf, PackageId)>,
    /// The modules in this workspace.
    pub(crate) modules: OrdMap<ModuleId, Arc<Module>>,
    /// Errors discovered while building this workspace.
    pub(crate) errors: Vec<WorkspaceError>,
}

impl Workspace {
    /// Check if this is a monorepo workspace.
    pub fn is_monorepo(&self) -> bool {
        self.kind == WorkspaceKind::Monorepo
    }

    /// Check if a path is within this workspace.
    pub fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    /// Return the package ids in this workspace.
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.packages.keys().copied()
    }

    /// Return one package for one package id.
    pub(crate) fn package(&self, package_id: PackageId) -> Option<Arc<Package>> {
        self.packages.get(&package_id).cloned()
    }

    /// Return the package roots in this workspace.
    pub(crate) fn package_roots(&self) -> &[(PathBuf, PackageId)] {
        &self.package_roots
    }

    /// Return the nearest package for one path.
    pub(crate) fn package_for_path(&self, path: &Path) -> Option<Arc<Package>> {
        for (package_root, package_id) in &self.package_roots {
            if path.starts_with(package_root) {
                return self.packages.get(package_id).cloned();
            }
        }

        None
    }

    /// Return one indexed explicit target by id.
    pub(crate) fn target_by_id(&self, target_id: TargetId) -> Option<&Target> {
        self.packages
            .get(&target_id.package_id())
            .and_then(|package| package.targets.get(&target_id))
    }

    /// Return one module for one module id.
    pub(crate) fn module(&self, module_id: ModuleId) -> Option<Arc<Module>> {
        self.modules.get(&module_id).cloned()
    }

    /// Return all modules in this workspace.
    pub(crate) fn modules(&self) -> &OrdMap<ModuleId, Arc<Module>> {
        &self.modules
    }

    /// Return errors discovered while building this workspace.
    pub fn errors(&self) -> &[WorkspaceError] {
        &self.errors
    }
}
