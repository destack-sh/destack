use std::path::{Path, PathBuf};

use destack_source::{FileId, ModuleId, PackageId, TargetId};
use im::OrdMap;

use crate::{Module, Package, Target};

/// Kind of workspace based on how it was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorkspaceKind {
    /// Monorepo with multiple member projects.
    Monorepo,
    /// Single package workspace.
    #[default]
    SinglePackage,
}

/// A repository-derived workspace view for one revision.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The workspace config declaration file id when present.
    pub file_id: Option<FileId>,
    /// The workspace root directory.
    pub root: PathBuf,
    /// The workspace kind.
    pub kind: WorkspaceKind,

    /// The package snapshots in this workspace.
    pub(crate) packages: OrdMap<PackageId, Package>,
    /// The package roots ordered from most specific to least specific.
    pub(crate) package_paths: Vec<(PathBuf, PackageId)>,
    /// The module snapshots in this workspace.
    pub(crate) modules: OrdMap<ModuleId, Module>,
    /// Legacy target index retained for snapshot compatibility.
    pub(crate) targets: OrdMap<TargetId, Target>,
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

    /// Return one package snapshot for one package id.
    pub(crate) fn package(&self, package_id: PackageId) -> Option<&Package> {
        self.packages.get(&package_id)
    }

    /// Return the package paths in this workspace.
    pub(crate) fn package_paths(&self) -> &[(PathBuf, PackageId)] {
        &self.package_paths
    }

    /// Return the nearest package snapshot for one path.
    pub(crate) fn package_for_path(&self, path: &Path) -> Option<&Package> {
        for (package_path, package_id) in &self.package_paths {
            if path.starts_with(package_path) {
                return self.packages.get(package_id);
            }
        }

        None
    }

    /// Return one indexed explicit target by id.
    pub(crate) fn target_by_id(&self, target_id: TargetId) -> Option<&Target> {
        self.packages
            .get(&target_id.package_id())
            .and_then(|package| package.targets.get(&target_id))
            .or_else(|| self.targets.get(&target_id))
    }

    /// Return one module snapshot for one module id.
    pub(crate) fn module(&self, module_id: ModuleId) -> Option<&Module> {
        self.modules.get(&module_id)
    }

    /// Return all module snapshots in this workspace.
    pub(crate) fn modules(&self) -> &OrdMap<ModuleId, Module> {
        &self.modules
    }
}
