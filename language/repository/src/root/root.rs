use std::fmt;
use std::path::{Path, PathBuf};

use tspp_source::FileId;

/// How one repository root is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootKind {
    /// Root declared by a workspace manifest.
    Workspace,
    /// Root declared by a package manifest.
    Package,
    /// Source root discovered without a manifest.
    Loose,
}

impl fmt::Display for RootKind {
    /// Format the root kind for user-facing output.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace => formatter.write_str("workspace"),
            Self::Package => formatter.write_str("package"),
            Self::Loose => formatter.write_str("loose"),
        }
    }
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
        self.kind == RootKind::Workspace
    }

    /// Return true when a path is within this root.
    pub fn contains_path(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}
