use std::path::PathBuf;

use destack_source::{FileId, PackageId};

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
    /// The package ids in this workspace.
    pub package_ids: Vec<PackageId>,
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
}
