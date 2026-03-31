use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Destack;

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
    /// The workspace root directory.
    pub root: PathBuf,
    /// The workspace kind.
    pub kind: WorkspaceKind,
    /// The root `destack.json` configuration.
    pub config: Option<Arc<Destack>>,
    /// The package directories in this workspace.
    pub package_paths: Vec<PathBuf>,
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

    /// Find the package path that contains the given path.
    pub fn find_package_for_path(&self, path: &std::path::Path) -> Option<&PathBuf> {
        self.package_paths
            .iter()
            .find(|pkg_path| path.starts_with(pkg_path))
    }
}
