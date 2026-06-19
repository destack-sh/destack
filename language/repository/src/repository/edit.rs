use destack_source::{Content, FileId};

use crate::repository::{Repository, RepositoryError, Revision, normalize_logical_path};

/// One atomic mutation inside one repository edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Add one file with one full content payload.
    AddFile {
        /// The workspace logical path.
        logical_path: String,
        content: Content,
    },
    /// Set one file with one full content payload.
    SetFile {
        /// The workspace logical path.
        logical_path: String,
        content: Content,
    },
    /// Remove one file from the revision file bindings.
    RemoveFile {
        /// The workspace logical path.
        logical_path: String,
    },
    /// Move one file within the revision file bindings.
    MoveFile {
        /// The source workspace logical path.
        from: String,
        /// The destination workspace logical path.
        to: String,
    },
}

impl Edit {
    /// Build one text add edit.
    pub fn add_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::AddFile {
            logical_path: normalize_logical_path(path),
            content: Content::Text {
                content: content.into(),
            },
        }
    }

    /// Build one text set edit.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path(path),
            content: Content::Text {
                content: content.into(),
            },
        }
    }

    /// Build one remove edit.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::RemoveFile {
            logical_path: normalize_logical_path(path),
        }
    }

    /// Build one move edit.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::MoveFile {
            from: normalize_logical_path(from),
            to: normalize_logical_path(to),
        }
    }

    /// Return concrete file ids changed by this edit.
    pub fn changed_file_ids(&self) -> impl Iterator<Item = FileId> + '_ {
        let (first, second) = self.changed_logical_paths();
        let first = FileId::from_logical_str(first);
        let second = second.map(FileId::from_logical_str);

        [Some(first), second].into_iter().flatten()
    }

    /// Return logical file paths changed by this edit.
    pub(crate) fn changed_logical_paths(&self) -> (&str, Option<&str>) {
        match self {
            Self::AddFile { logical_path, .. }
            | Self::SetFile { logical_path, .. }
            | Self::RemoveFile { logical_path } => (logical_path, None),
            Self::MoveFile { from, to } => (from, Some(to)),
        }
    }
}

impl Repository {
    /// Commit repository edits against one base revision.
    pub fn commit_edits(
        &self,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Revision, RepositoryError> {
        if edits.is_empty() {
            Ok(revision)
        } else {
            self.fork_with_edits(revision, edits)
        }
    }
}
