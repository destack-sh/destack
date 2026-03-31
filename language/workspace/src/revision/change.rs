use destack_source::FileContent;

use crate::repository::normalize_logical_path_str;

/// One atomic source mutation inside one repository change.
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
    /// Remove one file from the revision source snapshot.
    RemoveFile {
        /// The workspace logical path.
        logical_path: String,
    },
    /// Move one file within the revision source snapshot.
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
            logical_path: normalize_logical_path_str(path.as_ref()),
            content: FileContent::Text {
                content: content.into(),
            },
        }
    }

    /// Build one text set edit.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path_str(path.as_ref()),
            content: FileContent::Text {
                content: content.into(),
            },
        }
    }

    /// Build one remove edit.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::RemoveFile {
            logical_path: normalize_logical_path_str(path.as_ref()),
        }
    }

    /// Build one move edit.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::MoveFile {
            from: normalize_logical_path_str(from.as_ref()),
            to: normalize_logical_path_str(to.as_ref()),
        }
    }
}

/// One ordered batch of source edits published as one revision change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Change {
    /// The atomic edits in this change.
    pub edits: Vec<Edit>,
}

impl Change {
    /// Build one change from explicit edits.
    pub fn new(edits: Vec<Edit>) -> Self {
        Self { edits }
    }

    /// Build one empty change.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build one single-edit change.
    pub fn single(edit: Edit) -> Self {
        Self { edits: vec![edit] }
    }

    /// Build one single-file add change.
    pub fn add_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::single(Edit::add_text(path, content))
    }

    /// Build one single-file set change.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::single(Edit::set_text(path, content))
    }

    /// Build one single-file remove change.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::single(Edit::remove_file(path))
    }

    /// Build one single-file move change.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::single(Edit::move_file(from, to))
    }

    /// Return the edits in this change.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }
}

impl From<Edit> for Change {
    fn from(edit: Edit) -> Self {
        Self::single(edit)
    }
}

impl From<Vec<Edit>> for Change {
    fn from(edits: Vec<Edit>) -> Self {
        Self::new(edits)
    }
}

impl<const N: usize> From<[Edit; N]> for Change {
    fn from(edits: [Edit; N]) -> Self {
        Self::new(Vec::from(edits))
    }
}
