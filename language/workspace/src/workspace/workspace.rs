use std::path::PathBuf;

use destack_source::{FileId, ModuleId, PackageId};
use im::OrdMap;

use crate::{Module, Package};

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
    pub destack_file_id: Option<FileId>,
    /// The workspace root directory.
    pub root: PathBuf,
    /// The workspace kind.
    pub kind: WorkspaceKind,
    /// The discovered packages in this workspace.
    pub(crate) packages: OrdMap<PackageId, Package>,
    /// The discovered modules in this workspace.
    pub(crate) modules: OrdMap<ModuleId, Module>,
}

impl Workspace {
    /// Check if this is a monorepo workspace.
    pub fn is_monorepo(&self) -> bool {
        self.kind == WorkspaceKind::Monorepo
    }

    /// Check if a path is within this workspace.
    pub fn contains_path(&self, path: &std::path::Path) -> bool {
        path.starts_with(&self.root)
    }

    /// Return the package ids in this workspace.
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.packages.keys().copied()
    }

    /// Return one discovered package for one package id.
    pub(crate) fn package(&self, package_id: PackageId) -> Option<&Package> {
        self.packages.get(&package_id)
    }

    /// Return one discovered module for one module id.
    pub(crate) fn module(&self, module_id: ModuleId) -> Option<&Module> {
        self.modules.get(&module_id)
    }

    /// Return all discovered modules in this workspace.
    pub(crate) fn modules(&self) -> &OrdMap<ModuleId, Module> {
        &self.modules
    }
}
