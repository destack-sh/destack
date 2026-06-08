use std::path::{Path, PathBuf};

use destack_source::FileId;

/// Kind of root based on how it was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RootKind {
    /// Monorepo with multiple member projects.
    Monorepo,
    /// Single package root.
    #[default]
    SinglePackage,
}

/// Root metadata derived for one revision.
#[derive(Debug, Clone)]
pub struct Root {
    /// The root config declaration file id when present.
    pub file_id: Option<FileId>,
    /// The root directory.
    pub root: PathBuf,
    /// The root kind.
    pub kind: RootKind,
}

impl Root {
    /// Return true when this root contains multiple packages.
    pub fn is_monorepo(&self) -> bool {
        self.kind == RootKind::Monorepo
    }

    /// Return true when a path is within this root.
    pub fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}
