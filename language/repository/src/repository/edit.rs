use std::collections::HashSet;

use destack_source::{FileContent, FileId};

use crate::repository::{Repository, RepositoryError, Revision, normalize_logical_path};

/// One atomic mutation inside one repository edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Add one file with one full content payload.
    AddFile {
        /// The workspace logical path.
        logical_path: String,
        content: FileContent,
    },
    /// Set one file with one full content payload.
    SetFile {
        /// The workspace logical path.
        logical_path: String,
        content: FileContent,
    },
    /// Remove one file from the revision file map.
    RemoveFile {
        /// The workspace logical path.
        logical_path: String,
    },
    /// Move one file within the revision file map.
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
            content: FileContent::Text {
                content: content.into(),
            },
        }
    }

    /// Build one text set edit.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path(path),
            content: FileContent::Text {
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
}

/// Repository edits plus the files they affect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryChange {
    /// Repository edits.
    edits: Vec<Edit>,
    /// Affected file ids.
    file_ids: Vec<FileId>,
    /// File ids already recorded in this change.
    seen_file_ids: HashSet<FileId>,
}

impl RepositoryChange {
    /// Create an empty repository change.
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            file_ids: Vec::new(),
            seen_file_ids: HashSet::new(),
        }
    }

    /// Build one repository change from one edit.
    pub fn from_edit(edit: Edit) -> Self {
        let mut change = Self::new();
        change.push(edit);

        change
    }

    /// Return true when this change has no edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Return the file ids affected by this change.
    pub fn file_ids(&self) -> &[FileId] {
        &self.file_ids
    }

    /// Push one repository edit.
    pub fn push(&mut self, edit: Edit) {
        let file_id = edit_file_id(&edit);
        if self.seen_file_ids.insert(file_id) {
            self.file_ids.push(file_id);
        }

        self.edits.push(edit);
    }

    /// Append another repository change.
    pub fn extend(&mut self, change: RepositoryChange) {
        for edit in change.edits {
            self.push(edit);
        }
    }
}

impl Default for RepositoryChange {
    /// Create an empty repository change.
    fn default() -> Self {
        Self::new()
    }
}

impl Repository {
    /// Commit one repository change against one base revision.
    pub fn commit_change(
        &self,
        revision: Revision,
        change: RepositoryChange,
    ) -> Result<Revision, RepositoryError> {
        if change.is_empty() {
            Ok(revision)
        } else {
            self.fork_with_edits(revision, change.edits)
        }
    }
}

/// Return one file id affected by an edit.
fn edit_file_id(edit: &Edit) -> FileId {
    match edit {
        Edit::AddFile { logical_path, .. }
        | Edit::SetFile { logical_path, .. }
        | Edit::RemoveFile { logical_path } => FileId::from_logical_str(logical_path),
        Edit::MoveFile { to, .. } => FileId::from_logical_str(to),
    }
}
