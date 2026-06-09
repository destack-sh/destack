use crate::bridge;

use super::Edit;

/// Source input used to open a live session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Filesystem source rooted at a path.
    FileSystem {
        /// Source root or child path.
        path: String,
    },
    /// In-memory filesystem source seeded by edits.
    Memory {
        /// Source root path used for repository identity.
        root: String,
        /// Edits used to seed the memory filesystem.
        edits: Vec<Edit>,
    },
}

impl Source {
    /// Create one filesystem source input.
    pub fn file_system(path: impl Into<String>) -> Self {
        Self::FileSystem { path: path.into() }
    }

    /// Create one memory source input.
    pub fn memory(root: impl Into<String>, edits: Vec<Edit>) -> Self {
        Self::Memory {
            root: root.into(),
            edits,
        }
    }
}
